//! Bridge from the in-game preview back to the launcher executable.
//!
//! The preview remains visual-only: Play does not load a world or connect to a server. It opens
//! the Legohan's Client launcher binary shipped next to the preview executable, so the player can
//! continue from the real launcher UI.

use std::{
    env,
    ffi::OsStr,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

/// Launcher executable names, in preference order.
const LAUNCHER_NAMES: [&str; 4] = [
    "legohans-client.exe",
    "legohans-client",
    "lc-app.exe",
    "lc-app",
];

/// Find a launcher executable in `dir`.
fn find_launcher_in(dir: &Path) -> Option<PathBuf> {
    LAUNCHER_NAMES
        .iter()
        .map(|name| dir.join(name))
        .find(|candidate| candidate.is_file())
}

/// Locate the launcher binary that belongs to this preview build.
///
/// A sibling no-console launcher is preferred when available. Otherwise the current executable is
/// used when it already has a launcher name, which covers running the preview directly.
pub fn launcher_path() -> Result<PathBuf, String> {
    let current = env::current_exe()
        .map_err(|err| format!("Could not read the current executable path: {err}"))?;
    let dir = current
        .parent()
        .ok_or_else(|| "The current executable has no parent directory.".to_string())?;

    if let Some(launcher) = find_launcher_in(dir) {
        return Ok(launcher);
    }

    if current
        .file_name()
        .and_then(OsStr::to_str)
        .is_some_and(|name| LAUNCHER_NAMES.contains(&name))
    {
        return Ok(current);
    }

    Err(format!(
        "No launcher executable was found in {}.",
        dir.display()
    ))
}

/// Start the launcher and let the preview window continue running.
pub fn open_launcher() -> Result<(), String> {
    let path = launcher_path()?;
    Command::new(&path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|err| format!("Could not start {}: {err}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn launcher_names_are_recognized() {
        assert!(LAUNCHER_NAMES.contains(&"legohans-client.exe"));
        assert!(LAUNCHER_NAMES.contains(&"lc-app.exe"));
    }

    #[test]
    fn prefers_no_console_launcher_when_present() {
        let dir = env::temp_dir().join(format!("lc-launch-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        for name in ["lc-app.exe", "legohans-client.exe"] {
            std::fs::write(dir.join(name), b"stub").unwrap();
        }
        let found = find_launcher_in(&dir).unwrap();
        assert_eq!(found.file_name().unwrap(), "legohans-client.exe");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn reports_missing_launcher() {
        let dir = env::temp_dir().join(format!("lc-launch-missing-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        assert!(find_launcher_in(&dir).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
