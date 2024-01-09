use anyhow::{bail, Context, Result};
use log::{debug, error};
use magic::cookie::{Cookie, Flags, Load};
use std::path::Path;

thread_local! {
    static COOKIE: Option<Cookie<Load>> = load_cookie();
}

fn load_cookie() -> Option<Cookie<Load>> {
    let cookie = match Cookie::open(Flags::MIME_TYPE) {
        Ok(cookie) => match cookie.load(&Default::default()) {
            Ok(cookie) => Some(cookie),
            Err(err) => {
                error!("Failed to load default magic database file: {}", err);
                None
            }
        },
        Err(err) => {
            error!("Failed to open magic cookie: {}", err);
            None
        }
    }?;

    debug!("Loaded magic cookie");
    Some(cookie)
}

fn read_mime_type(cookie: &Cookie<Load>, path: &Path) -> Result<MimeType> {
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
