//! Resource pack import, inspection and ordering.

use std::fmt;
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::util::component_text;

const META_LIMIT: u64 = 1024 * 1024;
const ICON_LIMIT: u64 = 2 * 1024 * 1024;
const MAX_COPY_ENTRIES: usize = 50_000;
const MAX_COPY_DEPTH: usize = 16;

/// Whether a pack is a zip file or an unpacked folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackKind {
    /// A `.zip` archive.
    Zip,
    /// A folder.
    Folder,
}

/// Why a pack could not be imported or read.
#[derive(Debug)]
pub enum PackError {
    /// Not a zip file or folder, or `pack.mcmeta` is missing at its root.
    NotAPack(String),
    /// `pack.mcmeta` is not valid JSON of the expected shape.
    InvalidMeta(String),
    /// A file system problem.
    Io(io::Error),
}

impl fmt::Display for PackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotAPack(why) => write!(f, "Not a resource pack: {why}"),
            Self::InvalidMeta(why) => write!(f, "pack.mcmeta is invalid: {why}"),
            Self::Io(e) => write!(f, "{e}"),
        }
    }
}

impl From<io::Error> for PackError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

/// Metadata read from a pack.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackInfo {
    /// File or folder name inside the packs directory.
    pub file_name: String,
    /// Zip or folder.
    pub kind: PackKind,
    /// Plain-text description.
    pub description: String,
    /// Declared pack format (or format range), for display only.
    pub format_label: Option<String>,
    /// PNG bytes of `pack.png`, if present.
    pub icon_png: Option<Vec<u8>>,
    /// Size on disk in bytes.
    pub size_bytes: u64,
}

fn format_value(v: &Value) -> Option<String> {
    match v {
        Value::Number(n) => Some(n.to_string()),
        Value::Array(items) => {
            let parts: Vec<String> = items
                .iter()
                .filter_map(|x| x.as_u64().map(|n| n.to_string()))
                .collect();
            (!parts.is_empty()).then(|| parts.join("."))
        }
        _ => None,
    }
}

/// Parse `pack.mcmeta` text into `(description, format label)`.
pub fn parse_mcmeta(text: &str) -> Result<(String, Option<String>), PackError> {
    let root: Value =
        serde_json::from_str(text).map_err(|e| PackError::InvalidMeta(e.to_string()))?;
    let pack = root
        .get("pack")
        .and_then(Value::as_object)
        .ok_or_else(|| PackError::InvalidMeta("missing \"pack\" section".to_string()))?;
    let description = pack
        .get("description")
        .map(component_text)
        .unwrap_or_default();
    let label = if let Some(f) = pack.get("pack_format").and_then(format_value) {
        Some(f)
    } else {
        let min = pack.get("min_format").and_then(format_value);
        let max = pack.get("max_format").and_then(format_value);
        match (min, max) {
            (Some(a), Some(b)) if a != b => Some(format!("{a}\u{2013}{b}")),
            (Some(a), _) | (None, Some(a)) => Some(a),
            _ => None,
        }
    };
    Ok((description, label))
}

fn read_limited(mut reader: impl Read, limit: u64) -> io::Result<Vec<u8>> {
    let mut buf = Vec::new();
    reader.by_ref().take(limit).read_to_end(&mut buf)?;
    Ok(buf)
}

fn dir_size(path: &Path, depth: usize, budget: &mut usize) -> u64 {
    if depth > MAX_COPY_DEPTH || *budget == 0 {
        return 0;
    }
    let Ok(entries) = fs::read_dir(path) else {
        return 0;
    };
    let mut total = 0;
    for entry in entries.flatten() {
        if *budget == 0 {
            break;
        }
        *budget -= 1;
        let Ok(meta) = entry.metadata() else { continue };
        if meta.is_dir() {
            total += dir_size(&entry.path(), depth + 1, budget);
        } else if meta.is_file() {
            total += meta.len();
        }
    }
    total
}

/// Read a pack's metadata from a `.zip` file or a folder.
pub fn inspect(path: &Path) -> Result<PackInfo, PackError> {
    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .ok_or_else(|| PackError::NotAPack("no file name".to_string()))?;
    let meta = fs::metadata(path)?;
    if meta.is_dir() {
        let mcmeta = path.join("pack.mcmeta");
        let size = fs::metadata(&mcmeta)
            .map_err(|_| PackError::NotAPack("pack.mcmeta not found in the folder".to_string()))?
            .len();
        if size > META_LIMIT {
            return Err(PackError::InvalidMeta("file is too large".to_string()));
        }
        let text = fs::read_to_string(&mcmeta)?;
        let (description, format_label) = parse_mcmeta(&text)?;
        let icon_png = match fs::metadata(path.join("pack.png")) {
            Ok(m) if m.is_file() && m.len() <= ICON_LIMIT => fs::read(path.join("pack.png")).ok(),
            _ => None,
        };
        let mut budget = MAX_COPY_ENTRIES;
        Ok(PackInfo {
            file_name,
            kind: PackKind::Folder,
            description,
            format_label,
            icon_png,
            size_bytes: dir_size(path, 0, &mut budget),
        })
    } else {
        let is_zip = path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("zip"));
        if !is_zip {
            return Err(PackError::NotAPack(
                "expected a .zip file or a folder".to_string(),
            ));
        }
        let mut archive = zip::ZipArchive::new(File::open(path)?)
            .map_err(|e| PackError::NotAPack(format!("not a readable zip file ({e})")))?;
        let text = {
            let entry = archive.by_name("pack.mcmeta").map_err(|_| {
                PackError::NotAPack("pack.mcmeta not found at the root of the zip".to_string())
            })?;
            String::from_utf8(read_limited(entry, META_LIMIT)?)
                .map_err(|_| PackError::InvalidMeta("not valid UTF-8".to_string()))?
        };
        let (description, format_label) = parse_mcmeta(&text)?;
        let icon_png = match archive.by_name("pack.png") {
            Ok(entry) => read_limited(entry, ICON_LIMIT).ok(),
            Err(_) => None,
        };
        Ok(PackInfo {
            file_name,
            kind: PackKind::Zip,
            description,
            format_label,
            icon_png,
            size_bytes: meta.len(),
        })
    }
}

fn unique_destination(dir: &Path, name: &str) -> PathBuf {
    let candidate = dir.join(name);
    if !candidate.exists() {
        return candidate;
    }
    let path = Path::new(name);
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| name.to_string());
    let ext = path.extension().map(|e| e.to_string_lossy().into_owned());
    for n in 2.. {
        let numbered = match &ext {
            Some(ext) => format!("{stem} ({n}).{ext}"),
            None => format!("{stem} ({n})"),
        };
        let candidate = dir.join(numbered);
        if !candidate.exists() {
            return candidate;
        }
    }
    unreachable!("an unused name always exists")
}

fn copy_dir(src: &Path, dst: &Path, depth: usize, budget: &mut usize) -> Result<(), PackError> {
    if depth > MAX_COPY_DEPTH {
        return Err(PackError::NotAPack(
            "folder is nested too deeply".to_string(),
        ));
    }
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        if *budget == 0 {
            return Err(PackError::NotAPack(
                "folder contains too many files".to_string(),
            ));
        }
        *budget -= 1;
        let file_type = entry.file_type()?;
        let target = dst.join(entry.file_name());
        if file_type.is_symlink() {
            continue;
        } else if file_type.is_dir() {
            copy_dir(&entry.path(), &target, depth + 1, budget)?;
        } else if file_type.is_file() {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

/// Validate `src` and copy it into `packs_dir`. Returns the stored file name.
pub fn import(src: &Path, packs_dir: &Path) -> Result<String, PackError> {
    let info = inspect(src)?;
    fs::create_dir_all(packs_dir)?;
    let dest = unique_destination(packs_dir, &info.file_name);
    match info.kind {
        PackKind::Zip => {
            fs::copy(src, &dest)?;
        }
        PackKind::Folder => {
            let mut budget = MAX_COPY_ENTRIES;
            if let Err(e) = copy_dir(src, &dest, 0, &mut budget) {
                let _ = fs::remove_dir_all(&dest);
                return Err(e);
            }
        }
    }
    Ok(dest
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or(info.file_name))
}

/// Delete an imported pack. `file_name` must be a plain name without path separators.
pub fn remove(packs_dir: &Path, file_name: &str) -> io::Result<()> {
    if file_name.is_empty()
        || file_name == "."
        || file_name == ".."
        || file_name.contains(['/', '\\'])
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid pack name",
        ));
    }
    let path = packs_dir.join(file_name);
    if path.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
}

/// One pack in priority order and whether it is enabled.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackRecord {
    /// File or folder name inside the packs directory.
    pub file_name: String,
    /// Whether the pack is enabled.
    pub enabled: bool,
}

/// The persisted pack order and enabled state.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PackStore {
    /// Packs from lowest to highest priority position in the list (top of the list wins).
    pub records: Vec<PackRecord>,
}

impl PackStore {
    /// Reconcile the records with what is actually on disk: drop missing packs and append new
    /// ones as disabled.
    pub fn sync(&mut self, packs_dir: &Path) {
        let mut on_disk: Vec<String> = fs::read_dir(packs_dir)
            .map(|rd| {
                rd.flatten()
                    .filter(|e| {
                        e.path().is_dir()
                            || e.path()
                                .extension()
                                .is_some_and(|x| x.eq_ignore_ascii_case("zip"))
                    })
                    .map(|e| e.file_name().to_string_lossy().into_owned())
                    .collect()
            })
            .unwrap_or_default();
        on_disk.sort();
        self.records.retain(|r| on_disk.contains(&r.file_name));
        for name in on_disk {
            if !self.records.iter().any(|r| r.file_name == name) {
                self.records.push(PackRecord {
                    file_name: name,
                    enabled: false,
                });
            }
        }
    }

    /// Enable or disable a pack.
    pub fn set_enabled(&mut self, file_name: &str, enabled: bool) {
        if let Some(r) = self.records.iter_mut().find(|r| r.file_name == file_name) {
            r.enabled = enabled;
        }
    }

    /// Move a pack one position up (towards higher priority).
    pub fn move_up(&mut self, index: usize) {
        if index > 0 && index < self.records.len() {
            self.records.swap(index, index - 1);
        }
    }

    /// Move a pack one position down.
    pub fn move_down(&mut self, index: usize) {
        if index + 1 < self.records.len() {
            self.records.swap(index, index + 1);
        }
    }

    /// Drop the record for a removed pack.
    pub fn forget(&mut self, file_name: &str) {
        self.records.retain(|r| r.file_name != file_name);
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::test_support::TempDir;
    use std::io::Write;

    pub(crate) const MCMETA: &str =
        r#"{"pack":{"pack_format":75,"description":[{"text":"Hello "},{"text":"Pack"}]}}"#;

    pub(crate) fn make_zip(path: &Path, entries: &[(&str, &[u8])]) {
        let mut zip = zip::ZipWriter::new(File::create(path).unwrap());
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        for (name, data) in entries {
            zip.start_file(*name, options).unwrap();
            zip.write_all(data).unwrap();
        }
        zip.finish().unwrap();
    }

    #[test]
    fn mcmeta_parsing() {
        let (d, f) = parse_mcmeta(MCMETA).unwrap();
        assert_eq!(d, "Hello Pack");
        assert_eq!(f.as_deref(), Some("75"));

        let (_, f) =
            parse_mcmeta(r#"{"pack":{"description":"x","min_format":[75,0],"max_format":[84,1]}}"#)
                .unwrap();
        assert_eq!(f.as_deref(), Some("75.0\u{2013}84.1"));

        assert!(parse_mcmeta("{}").is_err());
        assert!(parse_mcmeta("nope").is_err());
    }

    #[test]
    fn inspects_zip_and_folder() {
        let dir = TempDir::new();
        let zip_path = dir.path().join("a.zip");
        make_zip(
            &zip_path,
            &[("pack.mcmeta", MCMETA.as_bytes()), ("pack.png", b"PNGDATA")],
        );
        let info = inspect(&zip_path).unwrap();
        assert_eq!(info.kind, PackKind::Zip);
        assert_eq!(info.description, "Hello Pack");
        assert_eq!(info.icon_png.as_deref(), Some(&b"PNGDATA"[..]));

        let folder = dir.path().join("folderpack");
        fs::create_dir_all(&folder).unwrap();
        fs::write(folder.join("pack.mcmeta"), MCMETA).unwrap();
        let info = inspect(&folder).unwrap();
        assert_eq!(info.kind, PackKind::Folder);
        assert!(info.icon_png.is_none());
        assert!(info.size_bytes > 0);
    }

    #[test]
    fn rejects_non_packs() {
        let dir = TempDir::new();
        let zip_path = dir.path().join("b.zip");
        make_zip(&zip_path, &[("readme.txt", b"hi")]);
        assert!(matches!(inspect(&zip_path), Err(PackError::NotAPack(_))));

        let txt = dir.path().join("c.txt");
        fs::write(&txt, "x").unwrap();
        assert!(matches!(inspect(&txt), Err(PackError::NotAPack(_))));

        let bad_zip = dir.path().join("d.zip");
        fs::write(&bad_zip, "not a zip").unwrap();
        assert!(matches!(inspect(&bad_zip), Err(PackError::NotAPack(_))));

        let empty = dir.path().join("emptydir");
        fs::create_dir_all(&empty).unwrap();
        assert!(matches!(inspect(&empty), Err(PackError::NotAPack(_))));
    }

    #[test]
    fn import_copies_and_avoids_overwriting() {
        let dir = TempDir::new();
        let src = dir.path().join("src.zip");
        make_zip(&src, &[("pack.mcmeta", MCMETA.as_bytes())]);
        let packs = dir.path().join("packs");
        assert_eq!(import(&src, &packs).unwrap(), "src.zip");
        assert_eq!(import(&src, &packs).unwrap(), "src (2).zip");
        assert!(packs.join("src.zip").is_file() && packs.join("src (2).zip").is_file());

        let folder = dir.path().join("fp");
        fs::create_dir_all(folder.join("assets")).unwrap();
        fs::write(folder.join("pack.mcmeta"), MCMETA).unwrap();
        fs::write(folder.join("assets").join("x.txt"), "x").unwrap();
        assert_eq!(import(&folder, &packs).unwrap(), "fp");
        assert!(packs.join("fp").join("assets").join("x.txt").is_file());
    }

    #[test]
    fn remove_deletes_and_rejects_paths() {
        let dir = TempDir::new();
        let src = dir.path().join("p.zip");
        make_zip(&src, &[("pack.mcmeta", MCMETA.as_bytes())]);
        let packs = dir.path().join("packs");
        let name = import(&src, &packs).unwrap();
        remove(&packs, &name).unwrap();
        assert!(!packs.join(&name).exists());
        assert!(remove(&packs, "../x").is_err());
        assert!(remove(&packs, "..").is_err());
        assert!(remove(&packs, "").is_err());
    }

    #[test]
    fn store_sync_order_and_enable() {
        let dir = TempDir::new();
        let packs = dir.path().join("packs");
        fs::create_dir_all(&packs).unwrap();
        for n in ["b.zip", "a.zip"] {
            make_zip(&packs.join(n), &[("pack.mcmeta", MCMETA.as_bytes())]);
        }
        fs::write(packs.join("ignored.txt"), "x").unwrap();

        let mut store = PackStore::default();
        store.sync(&packs);
        assert_eq!(
            store
                .records
                .iter()
                .map(|r| r.file_name.as_str())
                .collect::<Vec<_>>(),
            ["a.zip", "b.zip"]
        );
        assert!(store.records.iter().all(|r| !r.enabled));

        store.set_enabled("b.zip", true);
        store.move_up(1);
        assert_eq!(store.records[0].file_name, "b.zip");
        assert!(store.records[0].enabled);
        store.move_up(0);
        store.move_down(1);
        assert_eq!(store.records[0].file_name, "b.zip");

        fs::remove_file(packs.join("a.zip")).unwrap();
        store.sync(&packs);
        assert_eq!(store.records.len(), 1);
        store.forget("b.zip");
        assert!(store.records.is_empty());
    }
}
