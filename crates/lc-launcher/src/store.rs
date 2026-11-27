//! JSON persistence with atomic writes, backups and corrupt-file recovery.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::util::now_secs;

/// A loaded value plus an optional warning to show the user.
#[derive(Debug)]
pub struct Loaded<T> {
    /// The loaded value (defaults if the file was missing or unreadable).
    pub value: T,
    /// A human-readable problem, if any.
    pub warning: Option<String>,
    /// Whether it is safe to save over the source file. Read errors keep the original file
    /// untouched, so callers must not overwrite it with defaults.
    pub ok: bool,
}

/// The backup written next to every JSON file.
pub fn backup_path(path: &Path) -> PathBuf {
    path.with_extension("json.bak")
}

fn quarantine(path: &Path) -> Option<PathBuf> {
    let backup = path.with_extension(format!("corrupt-{}.json", now_secs()));
    fs::rename(path, &backup).ok().map(|()| backup)
}

/// Load JSON from `path`. A missing file yields defaults silently; a corrupt file is set aside
/// and, if a valid backup exists, recovered from it. An unreadable file yields defaults with a
/// warning and reports that it must not be overwritten.
pub fn load_json<T: DeserializeOwned + Default>(path: &Path) -> Loaded<T> {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    match fs::read_to_string(path) {
        Ok(text) => match serde_json::from_str(&text) {
            Ok(value) => Loaded {
                value,
                warning: None,
                ok: true,
            },
            Err(error) => {
                let backup = backup_path(path);
                if let Ok(backup_text) = fs::read_to_string(&backup)
                    && let Ok(value) = serde_json::from_str(&backup_text)
                {
                    let _ = quarantine(path);
                    let backup_name = backup
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    return Loaded {
                        value,
                        warning: Some(format!(
                            "{name} could not be read ({error}). It was restored from {backup_name}."
                        )),
                        ok: true,
                    };
                }
                let moved = quarantine(path);
                let warning = if let Some(backup) = moved {
                    format!(
                        "{name} could not be read ({error}). It was kept as {} and defaults were loaded.",
                        backup
                            .file_name()
                            .map(|n| n.to_string_lossy().into_owned())
                            .unwrap_or_default()
                    )
                } else {
                    format!("{name} could not be read ({error}). Defaults were loaded.")
                };
                Loaded {
                    value: T::default(),
                    warning: Some(warning),
                    ok: true,
                }
            }
        },
        Err(e) if e.kind() == io::ErrorKind::NotFound => Loaded {
            value: T::default(),
            warning: None,
            ok: true,
        },
        Err(e) => Loaded {
            value: T::default(),
            warning: Some(format!("Could not read {}: {e}", path.display())),
            ok: false,
        },
    }
}

/// Serialize `value` as pretty JSON to a temporary file, then rename it over `path`. The last
/// valid contents are kept in a `.json.bak` file so a damaged main file can be recovered.
pub fn save_json<T: Serialize>(path: &Path, value: &T) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let text = serde_json::to_string_pretty(value).map_err(io::Error::other)?;
    if path.is_file()
        && let Ok(existing) = fs::read_to_string(path)
        && serde_json::from_str::<serde_json::Value>(&existing).is_ok()
    {
        let _ = fs::write(backup_path(path), existing);
    }
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, text)?;
    fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDir;
    use serde::Deserialize;

    #[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
    #[serde(default)]
    struct Sample {
        name: String,
        count: u32,
    }

    #[test]
    fn missing_file_gives_defaults_without_warning() {
        let dir = TempDir::new();
        let loaded: Loaded<Sample> = load_json(&dir.path().join("nope.json"));
        assert_eq!(loaded.value, Sample::default());
        assert!(loaded.warning.is_none());
        assert!(loaded.ok);
    }

    #[test]
    fn save_then_load_roundtrips() {
        let dir = TempDir::new();
        let path = dir.path().join("a").join("b.json");
        let value = Sample {
            name: "x".into(),
            count: 7,
        };
        save_json(&path, &value).unwrap();
        let loaded: Loaded<Sample> = load_json(&path);
        assert_eq!(loaded.value, value);
        assert!(loaded.warning.is_none());
        assert!(loaded.ok);
        assert!(!path.with_extension("json.tmp").exists());
    }

    #[test]
    fn corrupt_file_is_set_aside_with_warning() {
        let dir = TempDir::new();
        let path = dir.path().join("c.json");
        fs::write(&path, "{ not json").unwrap();
        let loaded: Loaded<Sample> = load_json(&path);
        assert_eq!(loaded.value, Sample::default());
        assert!(loaded.warning.unwrap().contains("could not be read"));
        assert!(!path.exists());
        let backups = fs::read_dir(dir.path())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| e.file_name().to_string_lossy().contains("corrupt"))
            .count();
        assert_eq!(backups, 1);
    }

    #[test]
    fn corrupt_file_recovers_from_the_last_backup() {
        let dir = TempDir::new();
        let path = dir.path().join("accounts.json");
        save_json(
            &path,
            &Sample {
                name: "kept".into(),
                count: 3,
            },
        )
        .unwrap();
        save_json(
            &path,
            &Sample {
                name: "newer".into(),
                count: 4,
            },
        )
        .unwrap();
        fs::write(&path, "damaged").unwrap();

        let loaded: Loaded<Sample> = load_json(&path);
        assert_eq!(loaded.value.name, "kept");
        assert_eq!(loaded.value.count, 3);
        assert!(loaded.warning.unwrap().contains("restored from"));
        assert!(loaded.ok);
        assert!(!path.exists());
        assert!(backup_path(&path).is_file());
    }

    #[test]
    fn saving_keeps_the_previous_valid_contents() {
        let dir = TempDir::new();
        let path = dir.path().join("d.json");
        save_json(
            &path,
            &Sample {
                name: "old".into(),
                count: 1,
            },
        )
        .unwrap();
        save_json(
            &path,
            &Sample {
                name: "new".into(),
                count: 2,
            },
        )
        .unwrap();
        let backup: Sample =
            serde_json::from_str(&fs::read_to_string(backup_path(&path)).unwrap()).unwrap();
        assert_eq!(backup.name, "old");
    }

    #[test]
    fn unreadable_source_is_not_safe_to_overwrite() {
        let dir = TempDir::new();
        let loaded: Loaded<Sample> = load_json(dir.path());
        assert!(!loaded.ok);
        assert!(loaded.warning.is_some());
    }

    #[test]
    fn unknown_and_missing_fields_are_tolerated() {
        let dir = TempDir::new();
        let path = dir.path().join("e.json");
        fs::write(&path, r#"{"name":"n","extra":1}"#).unwrap();
        let loaded: Loaded<Sample> = load_json(&path);
        assert_eq!(loaded.value.name, "n");
        assert_eq!(loaded.value.count, 0);
        assert!(loaded.warning.is_none());
    }
}
