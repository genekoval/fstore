use crate::{
    db::ObjectError,
    error::{Error, Result},
};

use chrono::{DateTime, Duration, Local};
use std::{
    mem,
    ops::{Deref, DerefMut},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex, RwLock,
    },
};
use tokio::sync::Notify;
use uuid::Uuid;

const MAX_ERRORS: usize = 100;

#[derive(Debug, Default)]
struct Inner {
    started: DateTime<Local>,
    ended: RwLock<Option<DateTime<Local>>>,
    total: u64,
    completed: AtomicU64,
    errors: AtomicU64,
    messages: Mutex<Vec<ObjectError>>,
    notify: Notify,
}

#[derive(Clone, Debug, Default)]
pub struct Progress {
    inner: Arc<Inner>,
}

impl Progress {
    fn new(started: DateTime<Local>, total: u64) -> Self {
        Self {
            inner: Arc::new(Inner {
                started,
                total,
                ..Default::default()
            }),
        }
    }

    pub async fn finished(&self) -> bool {
        self.inner.notify.notified().await;
        self.completed() == self.total()
    }

    fn finish(&self) {
        *self.inner.ended.write().unwrap() = Some(Local::now());
        self.inner.notify.notify_waiters();
    }

    pub fn completed(&self) -> u64 {
        self.inner.completed.load(Ordering::Relaxed)
    }

    pub fn errors(&self) -> u64 {
        self.inner.errors.load(Ordering::Relaxed)
    }

    pub fn total(&self) -> u64 {
        self.inner.total
    }

    pub fn started(&self) -> DateTime<Local> {
        self.inner.started
    }

    pub fn ended(&self) -> Option<DateTime<Local>> {
        *self.inner.ended.read().unwrap()
    }

    pub fn elapsed(&self) -> Duration {
        self.ended().unwrap_or_else(Local::now) - self.inner.started
    }

    pub(crate) fn error(&self, id: Uuid, message: String) -> Vec<ObjectError> {
        self.inner.errors.fetch_add(1, Ordering::Relaxed);
        self.push_error(id, message)
    }

    pub(crate) fn clear_error(&self, id: Uuid) -> Vec<ObjectError> {
        self.push_error(id, "".into())
    }

    fn push_error(&self, id: Uuid, message: String) -> Vec<ObjectError> {
        let mut messages = self.inner.messages.lock().unwrap();

        messages.push(ObjectError {
            object_id: id,
            message,
        });

        if messages.len() > MAX_ERRORS {
            mem::take(messages.as_mut())
        } else {
            vec![]
        }
    }

    pub(crate) fn increment(&self) {
        self.inner.completed.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn messages(&self) -> Vec<ObjectError> {
        mem::take(self.inner.messages.lock().unwrap().deref_mut())
    }
}

#[derive(Clone, Debug, Default)]
pub struct Task {
    progress: Arc<RwLock<Option<Progress>>>,
}

impl Task {
    fn start(&self, progress: Progress) -> Result<()> {
        let mut existing = self.progress.write().unwrap();

        if existing.is_some() {
            return Err(Error::InProgress);
        }

        *existing = Some(progress);

        Ok(())
    }

    fn clear(&self) {
        *self.progress.write().unwrap() = None;
    }

    pub fn progress(&self) -> Option<Progress> {
        self.progress.read().unwrap().clone()
    }
}

#[derive(Debug)]
pub struct ProgressGuard {
    progress: Progress,
    task: Task,
}

impl ProgressGuard {
    pub fn new(
        started: DateTime<Local>,
        total: u64,
        task: Task,
    ) -> Result<Self> {
        let progress = Progress::new(started, total);
        task.start(progress.clone())?;

        Ok(Self { progress, task })
    }
}

impl Deref for ProgressGuard {
    type Target = Progress;

    fn deref(&self) -> &Self::Target {
        &self.progress
    }
}

impl Drop for ProgressGuard {
    fn drop(&mut self) {
        self.progress.finish();
        self.task.clear();
    }
}
