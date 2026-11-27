//! JSON persistence with atomic writes and corrupt-file recovery.

use std::fs;
use std::io;
use std::path::Path;

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
}

/// Load JSON from `path`. A missing file yields defaults silently; an unreadable or corrupt
/// file yields defaults plus a warning, and a corrupt file is set aside rather than overwritten.
pub fn load_json<T: DeserializeOwned + Default>(path: &Path) -> Loaded<T> {
    match fs::read_to_string(path) {
        Ok(text) => match serde_json::from_str(&text) {
            Ok(value) => Loaded {
                value,
                warning: None,
            },
            Err(error) => {
                let backup = path.with_extension(format!("corrupt-{}.json", now_secs()));
                let moved = fs::rename(path, &backup).is_ok();
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let warning = if moved {
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
                }
            }
        },
        Err(e) if e.kind() == io::ErrorKind::NotFound => Loaded {
            value: T::default(),
            warning: None,
        },
        Err(e) => Loaded {
            value: T::default(),
            warning: Some(format!("Could not read {}: {e}", path.display())),
        },
    }
}

/// Serialize `value` as pretty JSON to a temporary file, then rename it over `path`.
pub fn save_json<T: Serialize>(path: &Path, value: &T) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let text = serde_json::to_string_pretty(value).map_err(io::Error::other)?;
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
    fn unknown_and_missing_fields_are_tolerated() {
        let dir = TempDir::new();
        let path = dir.path().join("d.json");
        fs::write(&path, r#"{"name":"n","extra":1}"#).unwrap();
        let loaded: Loaded<Sample> = load_json(&path);
        assert_eq!(loaded.value.name, "n");
        assert_eq!(loaded.value.count, 0);
        assert!(loaded.warning.is_none());
    }
}
