use crate::error::{internal, Result};

use log::{debug, error};
use magic::cookie::{Flags, Load};
use std::path::Path;

type Cookie = magic::Cookie<Load>;

thread_local! {
    static COOKIE: Option<Cookie> = load_cookie(Flags::MIME_TYPE);
}

fn load_cookie(flags: Flags) -> Option<Cookie> {
    let cookie = match magic::Cookie::open(flags) {
        Ok(cookie) => match cookie.load(&Default::default()) {
            Ok(cookie) => Some(cookie),
            Err(err) => {
                error!("Failed to load default magic database file: {}", err);
                return None;
            }
        },
        Err(err) => {
            error!("Failed to open magic cookie: {}", err);
            return None;
        }
    }?;

    debug!("Loaded magic cookie with flags: {flags}");
    Some(cookie)
}

fn with_cookie<F, R>(f: F) -> Result<R>
where
    F: FnOnce(&Cookie) -> Result<R>,
{
    COOKIE.with(|cookie| match cookie {
        Some(ref cookie) => f(cookie),
        None => internal!("Magic cookie is missing"),
    })
}

pub struct MimeType {
    pub r#type: String,
    pub subtype: String,
}

fn read_mime_type(cookie: &Cookie, path: &Path) -> Result<MimeType> {
    let description = match cookie.file(path) {
        Ok(description) => description,
        Err(err) => internal!(
            "Failed to read textual description of contents of '{}': {}",
            path.display(),
            err
        ),
    };

    let read = |part: &str, value: Option<&str>| -> Result<String> {
        match value {
            Some(value) => Ok(value.to_string()),
            None => internal!(
                "Failed to read mime type of '{}': \
                Magic description '{}' does not contain {}",
                path.display(),
                description,
                part
            ),
        }
    };

    let mut mime = description.split('/');

    let r#type = read("type", mime.next())?;
    let subtype = read("subtype", mime.next())?;

    Ok(MimeType { r#type, subtype })
}

pub fn mime_type(path: &Path) -> Result<MimeType> {
    with_cookie(|cookie| read_mime_type(cookie, path))
}
