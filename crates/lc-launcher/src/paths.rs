//! Where the launcher keeps its files.

use std::path::{Path, PathBuf};

/// Directory layout of the launcher's data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LauncherDirs {
    root: PathBuf,
}

impl LauncherDirs {
    /// Use `root` as the launcher data directory.
    pub fn from_root(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Detect the platform default, honouring the `LC_HOME` override.
    pub fn detect() -> Self {
        let get = |key: &str| std::env::var(key).ok().filter(|v| !v.is_empty());
        Self::from_root(platform_root(std::env::consts::OS, get))
    }

    /// The data root.
    pub fn root(&self) -> &Path {
        &self.root
    }

    fn config(&self, name: &str) -> PathBuf {
        self.root.join("launcher").join(name)
    }

    /// `launcher/settings.json`
    pub fn settings_file(&self) -> PathBuf {
        self.config("settings.json")
    }
    /// `launcher/profiles.json`
    pub fn profiles_file(&self) -> PathBuf {
        self.config("profiles.json")
    }
    /// `launcher/accounts.json`
    pub fn accounts_file(&self) -> PathBuf {
        self.config("accounts.json")
    }
    /// `launcher/servers.json`
    pub fn servers_file(&self) -> PathBuf {
        self.config("servers.json")
    }
    /// `launcher/resourcepacks.json`
    pub fn packs_file(&self) -> PathBuf {
        self.config("resourcepacks.json")
    }
    /// Default parent of per-installation game directories.
    pub fn instances_dir(&self) -> PathBuf {
        self.root.join("instances")
    }
    /// Default game directory of the installation `id`.
    pub fn instance_dir(&self, id: &str) -> PathBuf {
        self.instances_dir().join(id)
    }
    /// Folder holding imported resource packs.
    pub fn resource_packs_dir(&self) -> PathBuf {
        self.root.join("resourcepacks")
    }
    /// Launcher-wide screenshots folder.
    pub fn screenshots_dir(&self) -> PathBuf {
        self.root.join("screenshots")
    }
    /// Folder holding imported customization assets.
    pub fn custom_dir(&self) -> PathBuf {
        self.root.join("custom")
    }
    /// Imported skin texture.
    pub fn skin_file(&self) -> PathBuf {
        self.custom_dir().join("skin.png")
    }
    /// Imported cape texture.
    pub fn cape_file(&self) -> PathBuf {
        self.custom_dir().join("cape.png")
    }
    /// Imported elytra texture.
    pub fn elytra_file(&self) -> PathBuf {
        self.custom_dir().join("elytra.png")
    }
    /// Folder holding the four custom panorama faces.
    pub fn custom_panorama_dir(&self) -> PathBuf {
        self.custom_dir().join("panorama")
    }
}

/// Compute the data root for `os` (as in `std::env::consts::OS`) using `get` to read variables.
pub fn platform_root(os: &str, get: impl Fn(&str) -> Option<String>) -> PathBuf {
    if let Some(custom) = get("LC_HOME") {
        return PathBuf::from(custom);
    }
    match os {
        "windows" => match get("APPDATA") {
            Some(appdata) => PathBuf::from(appdata).join("LegohansClient"),
            None => PathBuf::from(".legohans-client"),
        },
        "macos" => match get("HOME") {
            Some(home) => PathBuf::from(home)
                .join("Library")
                .join("Application Support")
                .join("LegohansClient"),
            None => PathBuf::from(".legohans-client"),
        },
        _ => match (get("XDG_DATA_HOME"), get("HOME")) {
            (Some(xdg), _) => PathBuf::from(xdg).join("legohans-client"),
            (None, Some(home)) => PathBuf::from(home)
                .join(".local")
                .join("share")
                .join("legohans-client"),
            _ => PathBuf::from(".legohans-client"),
        },
    }
}

/// Locate the user's own official Minecraft folder (`.minecraft`), if it can be guessed.
pub fn default_minecraft_dir(os: &str, get: impl Fn(&str) -> Option<String>) -> Option<PathBuf> {
    match os {
        "windows" => get("APPDATA").map(|a| PathBuf::from(a).join(".minecraft")),
        "macos" => get("HOME").map(|h| {
            PathBuf::from(h)
                .join("Library")
                .join("Application Support")
                .join("minecraft")
        }),
        _ => get("HOME").map(|h| PathBuf::from(h).join(".minecraft")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn env(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let map: HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect();
        move |k| map.get(k).cloned()
    }

    #[test]
    fn windows_uses_appdata() {
        let root = platform_root(
            "windows",
            env(&[("APPDATA", "C:\\Users\\a\\AppData\\Roaming")]),
        );
        assert!(root.ends_with("LegohansClient"));
        assert!(root.starts_with("C:\\Users\\a\\AppData\\Roaming"));
    }

    #[test]
    fn macos_uses_application_support() {
        let root = platform_root("macos", env(&[("HOME", "/Users/a")]));
        assert_eq!(
            root,
            PathBuf::from("/Users/a/Library/Application Support/LegohansClient")
        );
    }

    #[test]
    fn linux_prefers_xdg() {
        let root = platform_root("linux", env(&[("XDG_DATA_HOME", "/x"), ("HOME", "/h")]));
        assert_eq!(root, PathBuf::from("/x/legohans-client"));
        let root = platform_root("linux", env(&[("HOME", "/h")]));
        assert_eq!(root, PathBuf::from("/h/.local/share/legohans-client"));
    }

    #[test]
    fn override_wins() {
        let root = platform_root("windows", env(&[("LC_HOME", "/custom"), ("APPDATA", "/a")]));
        assert_eq!(root, PathBuf::from("/custom"));
    }

    #[test]
    fn minecraft_dir_guesses() {
        assert_eq!(
            default_minecraft_dir("linux", env(&[("HOME", "/h")])),
            Some(PathBuf::from("/h/.minecraft"))
        );
        assert!(default_minecraft_dir("linux", env(&[])).is_none());
    }

    #[test]
    fn layout_is_consistent() {
        let d = LauncherDirs::from_root("/r");
        assert_eq!(
            d.settings_file(),
            PathBuf::from("/r/launcher/settings.json")
        );
        assert_eq!(d.instance_dir("abc"), PathBuf::from("/r/instances/abc"));
    }
}
