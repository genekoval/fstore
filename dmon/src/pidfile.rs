use std::{
    fs::File, io::Write, os::unix::fs::PermissionsExt, path::Path, process,
};

pub fn create(path: &Path) -> Result<(), String> {
    let mut file = File::options()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|err| {
            format!("Failed to create PID file '{}': {err}", path.display())
        })?;

    writeln!(file, "{}", process::id()).map_err(|err| {
        format!("Failed to write PID to file '{}': {err}", path.display())
    })?;

    file.metadata()
        .map_err(|err| {
            format!(
                "Failed to fetch PID file metadata '{}': {err}",
                path.display()
            )
        })?
        .permissions()
        .set_mode(0o644);

    file.sync_all().map_err(|err| {
        format!(
            "Failed to sync PID file '{}' data to disk: {err}",
            path.display()
        )
    })?;

    Ok(())
}
