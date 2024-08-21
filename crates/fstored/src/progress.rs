use log::error;
use ratatui::{
    prelude::*,
    style::palette::tailwind,
    widgets::{block::Title, Block, Borders, Gauge},
    TerminalOptions, Viewport,
};
use std::io::{self, Stdout};
use tokio::{
    task::{self, JoinHandle},
    time::{sleep, Duration},
};
use tokio_util::sync::CancellationToken;

const VIEWPORT_HEIGHT: u16 = 2;

#[derive(Clone, Copy, Debug)]
struct Progress<'a> {
    title: &'a str,
    completed: u64,
    total: u64,
}

impl<'a> Progress<'a> {
    fn render_gauge(&self, area: Rect, buf: &mut Buffer) {
        let completed = self.completed as f64;
        let total = self.total as f64;

        let ratio = completed / total;
        let percentage = (ratio * 100.0).round();

        let title = Block::default()
            .title(Title::from(self.title))
            .borders(Borders::NONE);

        Gauge::default()
            .gauge_style(tailwind::BLUE.c800)
            .ratio(ratio)
            .block(title)
            .label(format!("{completed}/{total} ({percentage}%)"))
            .render(area, buf);
    }
}

impl Widget for &Progress<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        self.render_gauge(area, buf);
    }
}

pub struct ProgressBar<'a> {
    title: &'a str,
    progress: fstore_core::Progress,
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl<'a> ProgressBar<'a> {
    pub fn new(
        title: &'a str,
        progress: fstore_core::Progress,
    ) -> Result<Self, String> {
        crossterm::terminal::enable_raw_mode().map_err(|err| {
            format!("failed to enable terminal raw mode: {err}")
        })?;

        let terminal = Terminal::with_options(
            CrosstermBackend::new(io::stdout()),
            TerminalOptions {
                viewport: Viewport::Inline(VIEWPORT_HEIGHT),
            },
        )
        .map_err(|err| format!("failed to initialize terminal: {err}"))?;

        Ok(Self {
            title,
            progress,
            terminal,
        })
    }

    fn update(&mut self) -> bool {
        if let Err(err) = self.draw(Progress {
            title: self.title,
            completed: self.progress.completed(),
            total: self.progress.total(),
        }) {
            error!("Failed to update progress bar: {err}");
            return false;
        }

        true
    }

    fn draw(&mut self, progress: Progress) -> Result<(), String> {
        self.terminal
            .draw(|frame| frame.render_widget(&progress, frame.area()))
            .map_err(|err| format!("failed to draw progress bar: {err}"))?;

        Ok(())
    }

    fn exit(&mut self) -> Result<(), String> {
        crossterm::terminal::disable_raw_mode().map_err(|err| {
            format!("failed to disable terminal raw mode: {err}")
        })?;

        self.terminal
            .clear()
            .map_err(|err| format!("failed to clear terminal: {err}"))?;

        Ok(())
    }
}

impl<'a> Drop for ProgressBar<'a> {
    fn drop(&mut self) {
        if let Err(err) = self.exit() {
            error!("Progress bar failed to exit properly: {err}");
        }
    }
}

pub struct ProgressBarTask {
    token: CancellationToken,
    handle: JoinHandle<()>,
}

impl ProgressBarTask {
    pub fn new(title: String, progress: fstore_core::Progress) -> Self {
        let token = CancellationToken::new();
        let cloned_token = token.clone();

        let handle = task::spawn(async move {
            let mut bar = match ProgressBar::new(title.as_str(), progress) {
                Ok(bar) => bar,
                Err(err) => {
                    error!("Failed to initialize progress bar: {err}");
                    return;
                }
            };

            loop {
                tokio::select! {
                    _ = cloned_token.cancelled() => {
                        break;
                    }
                    _ = sleep(Duration::from_millis(100)) => {
                        if !bar.update() {
                            return;
                        }
                    }
                }
            }

            bar.update();
            sleep(Duration::from_millis(500)).await;
        });

        Self { token, handle }
    }

    pub async fn cancel(self) {
        self.token.cancel();
        self.handle.await.unwrap();
    }
}
