use anyhow::{bail, Context, Result};
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

fn read_mime_type(cookie: &Cookie, path: &Path) -> Result<MimeType> {
    let description = cookie.file(path).with_context(|| {
        format!(
            "Failed to read textual description of contents of '{}'",
            path.display()
        )
    })?;

    let read = |part: &str, value: Option<&str>| -> Result<String> {
        match value {
            Some(value) => Ok(value.to_string()),
            None => bail!(
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

pub struct MimeType {
    pub r#type: String,
    pub subtype: String,
}

pub fn mime_type(path: &Path) -> Result<MimeType> {
    COOKIE.with(|cookie| match cookie {
        Some(ref cookie) => read_mime_type(cookie, path),
        None => bail!("Magic cookie is missing"),
    })
}
