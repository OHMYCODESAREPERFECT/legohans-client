//! Installation profiles.

use std::fmt;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::paths::LauncherDirs;
use crate::util::{new_id, now_secs};

/// Game versions this launcher can configure. The Rust client targets exactly one version.
pub const SUPPORTED_VERSIONS: &[&str] = &[lc_protocol::TARGET_GAME_VERSION];

/// Decorative icon of an installation card.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ProfileIcon {
    /// Grass block.
    #[default]
    Grass,
    /// Stone block.
    Stone,
    /// Obsidian block.
    Obsidian,
    /// Amethyst block.
    Amethyst,
    /// Diamond block.
    Diamond,
    /// Gold block.
    Gold,
}

impl ProfileIcon {
    /// All icons in display order.
    pub const ALL: [ProfileIcon; 6] = [
        Self::Grass,
        Self::Stone,
        Self::Obsidian,
        Self::Amethyst,
        Self::Diamond,
        Self::Gold,
    ];

    /// Display name.
    pub fn name(self) -> &'static str {
        match self {
            Self::Grass => "Grass",
            Self::Stone => "Stone",
            Self::Obsidian => "Obsidian",
            Self::Amethyst => "Amethyst",
            Self::Diamond => "Diamond",
            Self::Gold => "Gold",
        }
    }
}

/// Why a profile is invalid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileError {
    /// The name is empty.
    EmptyName,
    /// The name is longer than 32 characters.
    NameTooLong,
    /// The width is outside 320..=7680.
    BadWidth,
    /// The height is outside 240..=4320.
    BadHeight,
    /// The version is not supported.
    UnsupportedVersion(String),
    /// A custom game directory was requested but left empty.
    EmptyGameDir,
}

impl fmt::Display for ProfileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyName => write!(f, "Give the installation a name."),
            Self::NameTooLong => write!(f, "The name can be at most 32 characters."),
            Self::BadWidth => write!(f, "Width must be between 320 and 7680."),
            Self::BadHeight => write!(f, "Height must be between 240 and 4320."),
            Self::UnsupportedVersion(v) => write!(f, "Version {v} is not supported."),
            Self::EmptyGameDir => {
                write!(f, "Enter a game directory or turn the custom folder off.")
            }
        }
    }
}

/// An installation profile.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Profile {
    /// Unique id.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Free-form notes.
    pub notes: String,
    /// Game version.
    pub version: String,
    /// Card icon.
    pub icon: ProfileIcon,
    /// Custom game directory; `None` uses the default per-installation folder.
    pub game_dir: Option<String>,
    /// Window width in pixels.
    pub width: u32,
    /// Window height in pixels.
    pub height: u32,
    /// Start fullscreen.
    pub fullscreen: bool,
    /// Creation time (Unix seconds).
    pub created: u64,
    /// Last time the game was launched with this profile (Unix seconds).
    pub last_played: Option<u64>,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            notes: String::new(),
            version: lc_protocol::TARGET_GAME_VERSION.to_string(),
            icon: ProfileIcon::default(),
            game_dir: None,
            width: 1280,
            height: 720,
            fullscreen: false,
            created: 0,
            last_played: None,
        }
    }
}

impl Profile {
    /// Create a new profile with a fresh id.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: new_id("profile"),
            name: name.into(),
            created: now_secs(),
            ..Self::default()
        }
    }

    /// Check the profile's fields.
    pub fn validate(&self) -> Result<(), ProfileError> {
        let name = self.name.trim();
        if name.is_empty() {
            return Err(ProfileError::EmptyName);
        }
        if name.chars().count() > 32 {
            return Err(ProfileError::NameTooLong);
        }
        if !(320..=7680).contains(&self.width) {
            return Err(ProfileError::BadWidth);
        }
        if !(240..=4320).contains(&self.height) {
            return Err(ProfileError::BadHeight);
        }
        if !SUPPORTED_VERSIONS.contains(&self.version.as_str()) {
            return Err(ProfileError::UnsupportedVersion(self.version.clone()));
        }
        if matches!(&self.game_dir, Some(d) if d.trim().is_empty()) {
            return Err(ProfileError::EmptyGameDir);
        }
        Ok(())
    }

    /// The directory the game will use for this profile.
    pub fn resolved_game_dir(&self, dirs: &LauncherDirs) -> PathBuf {
        match &self.game_dir {
            Some(custom) if !custom.trim().is_empty() => PathBuf::from(custom.trim()),
            _ => dirs.instance_dir(&self.id),
        }
    }
}

/// The persisted list of profiles.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ProfileStore {
    /// All profiles in display order.
    pub profiles: Vec<Profile>,
}

impl ProfileStore {
    /// Make sure at least one profile exists.
    pub fn ensure_default(&mut self) {
        if self.profiles.is_empty() {
            let mut p = Profile::new("Minecraft 1.21.11");
            p.notes = "Default installation".to_string();
            self.profiles.push(p);
        }
    }

    /// Find a profile by id.
    pub fn find(&self, id: &str) -> Option<&Profile> {
        self.profiles.iter().find(|p| p.id == id)
    }

    /// Mutable lookup by id.
    pub fn find_mut(&mut self, id: &str) -> Option<&mut Profile> {
        self.profiles.iter_mut().find(|p| p.id == id)
    }

    /// Add a validated profile.
    pub fn add(&mut self, profile: Profile) -> Result<String, ProfileError> {
        profile.validate()?;
        let id = profile.id.clone();
        self.profiles.push(profile);
        Ok(id)
    }

    /// Replace the profile with the same id by a validated one.
    pub fn update(&mut self, profile: Profile) -> Result<(), ProfileError> {
        profile.validate()?;
        if let Some(slot) = self.find_mut(&profile.id) {
            *slot = profile;
        }
        Ok(())
    }

    /// Delete a profile. The last remaining profile cannot be deleted; returns whether it was removed.
    pub fn delete(&mut self, id: &str) -> bool {
        if self.profiles.len() <= 1 {
            return false;
        }
        let before = self.profiles.len();
        self.profiles.retain(|p| p.id != id);
        self.profiles.len() != before
    }

    /// Duplicate a profile under a new id; returns the new id.
    pub fn duplicate(&mut self, id: &str) -> Option<String> {
        let mut copy = self.find(id)?.clone();
        copy.id = new_id("profile");
        copy.name = format!("{} (copy)", copy.name.chars().take(25).collect::<String>());
        copy.created = now_secs();
        copy.last_played = None;
        copy.game_dir = None;
        let new = copy.id.clone();
        self.profiles.push(copy);
        Some(new)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_profile_targets_supported_version() {
        let p = Profile::new("Test");
        assert_eq!(p.version, "1.21.11");
        assert!(p.validate().is_ok());
    }

    #[test]
    fn validation_catches_each_problem() {
        let mut p = Profile::new("ok");
        p.name = "   ".into();
        assert_eq!(p.validate(), Err(ProfileError::EmptyName));
        p.name = "x".repeat(33);
        assert_eq!(p.validate(), Err(ProfileError::NameTooLong));
        p.name = "ok".into();
        p.width = 10;
        assert_eq!(p.validate(), Err(ProfileError::BadWidth));
        p.width = 800;
        p.height = 99_999;
        assert_eq!(p.validate(), Err(ProfileError::BadHeight));
        p.height = 600;
        p.version = "1.8".into();
        assert!(matches!(
            p.validate(),
            Err(ProfileError::UnsupportedVersion(_))
        ));
        p.version = "1.21.11".into();
        p.game_dir = Some(" ".into());
        assert_eq!(p.validate(), Err(ProfileError::EmptyGameDir));
    }

    #[test]
    fn game_dir_resolution() {
        let dirs = LauncherDirs::from_root("/r");
        let mut p = Profile::new("a");
        p.id = "abc".into();
        assert_eq!(
            p.resolved_game_dir(&dirs),
            PathBuf::from("/r/instances/abc")
        );
        p.game_dir = Some("/custom/dir".into());
        assert_eq!(p.resolved_game_dir(&dirs), PathBuf::from("/custom/dir"));
    }

    #[test]
    fn store_creates_default_once() {
        let mut s = ProfileStore::default();
        s.ensure_default();
        s.ensure_default();
        assert_eq!(s.profiles.len(), 1);
    }

    #[test]
    fn add_update_delete_duplicate() {
        let mut s = ProfileStore::default();
        s.ensure_default();
        let id = s.add(Profile::new("Second")).unwrap();
        assert_eq!(s.profiles.len(), 2);

        let mut edited = s.find(&id).unwrap().clone();
        edited.name = "Renamed".into();
        s.update(edited).unwrap();
        assert_eq!(s.find(&id).unwrap().name, "Renamed");

        let copy = s.duplicate(&id).unwrap();
        assert_eq!(s.find(&copy).unwrap().name, "Renamed (copy)");
        assert_ne!(copy, id);

        assert!(s.delete(&copy));
        assert!(s.delete(&id));
        assert!(
            !s.delete(&s.profiles[0].id.clone()),
            "last profile must stay"
        );
    }

    #[test]
    fn invalid_profile_is_rejected_by_store() {
        let mut s = ProfileStore::default();
        assert!(s.add(Profile::new("")).is_err());
        assert!(s.profiles.is_empty());
    }

    #[test]
    fn serde_roundtrip() {
        let mut s = ProfileStore::default();
        s.ensure_default();
        let json = serde_json::to_string(&s).unwrap();
        let back: ProfileStore = serde_json::from_str(&json).unwrap();
        assert_eq!(s, back);
    }
}
