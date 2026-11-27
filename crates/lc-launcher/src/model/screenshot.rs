//! Screenshot discovery.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

/// A screenshot file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScreenshotEntry {
    /// Full path.
    pub path: PathBuf,
    /// File name.
    pub name: String,
    /// Modification time (Unix seconds).
    pub modified: u64,
    /// Size in bytes.
    pub size: u64,
}

/// Collect `.png` files directly inside `dirs`, newest first, at most `limit` entries.
pub fn scan(dirs: &[PathBuf], limit: usize) -> Vec<ScreenshotEntry> {
    let mut found = Vec::new();
    for dir in dirs {
        let Ok(entries) = fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !is_png(&path) {
                continue;
            }
            let Ok(meta) = entry.metadata() else { continue };
            if !meta.is_file() {
                continue;
            }
            let modified = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            found.push(ScreenshotEntry {
                name: entry.file_name().to_string_lossy().into_owned(),
                path,
                modified,
                size: meta.len(),
            });
        }
    }
    found.sort_by(|a, b| {
        b.modified
            .cmp(&a.modified)
            .then_with(|| a.name.cmp(&b.name))
    });
    found.truncate(limit);
    found
}

fn is_png(path: &Path) -> bool {
    path.extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("png"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDir;

    #[test]
    fn scans_pngs_only_and_respects_limit() {
        let dir = TempDir::new();
        let a = dir.path().join("a");
        let b = dir.path().join("b");
        fs::create_dir_all(&a).unwrap();
        fs::create_dir_all(&b).unwrap();
        fs::write(a.join("one.png"), "x").unwrap();
        fs::write(a.join("two.PNG"), "x").unwrap();
        fs::write(a.join("notes.txt"), "x").unwrap();
        fs::write(b.join("three.png"), "x").unwrap();
        fs::create_dir_all(b.join("dir.png")).unwrap();

        let all = scan(&[a.clone(), b.clone(), dir.path().join("missing")], 10);
        let mut names: Vec<_> = all.iter().map(|s| s.name.as_str()).collect();
        names.sort();
        assert_eq!(names, ["one.png", "three.png", "two.PNG"]);
        assert_eq!(scan(&[a, b], 2).len(), 2);
    }
}
