//! Persisted launcher data.

pub mod account;
pub mod color;
pub mod profile;
pub mod resource_pack;
pub mod screenshot;
pub mod server;
pub mod settings;
pub mod skin;

use std::path::PathBuf;

use serde::Serialize;

use crate::paths::LauncherDirs;
use crate::store::{load_json, save_json};
use account::{AccountRecord, AccountStore};
use profile::{Profile, ProfileStore};
use resource_pack::PackStore;
use server::ServerStore;
use settings::Settings;

#[derive(Debug, Default)]
struct Snapshots {
    settings: String,
    profiles: String,
    accounts: String,
    servers: String,
    packs: String,
}

/// All persisted launcher data plus the bookkeeping needed to save it.
#[derive(Debug)]
pub struct LauncherData {
    /// Where the data lives.
    pub dirs: LauncherDirs,
    /// Launcher settings.
    pub settings: Settings,
    /// Installation profiles.
    pub profiles: ProfileStore,
    /// Accounts.
    pub accounts: AccountStore,
    /// Saved servers.
    pub servers: ServerStore,
    /// Resource pack order and state.
    pub packs: PackStore,
    warnings: Vec<String>,
    snapshots: Snapshots,
    settings_loaded: bool,
    core_loaded: bool,
    accounts_loaded: bool,
}

fn save_if_changed<T: Serialize>(
    path: PathBuf,
    value: &T,
    snapshot: &mut String,
    errors: &mut Vec<String>,
) {
    let Ok(json) = serde_json::to_string(value) else {
        return;
    };
    if *snapshot == json {
        return;
    }
    *snapshot = json;
    if let Err(e) = save_json(&path, value) {
        errors.push(format!("Could not save {}: {e}", path.display()));
    }
}

impl LauncherData {
    /// Load only the settings (needed to decide how startup behaves).
    pub fn load_settings(dirs: LauncherDirs) -> Self {
        let mut warnings = Vec::new();
        let loaded = load_json::<Settings>(&dirs.settings_file());
        let settings_loaded = loaded.ok;
        warnings.extend(loaded.warning);
        let mut settings = loaded.value;
        settings.sanitize();
        Self {
            dirs,
            settings,
            profiles: ProfileStore::default(),
            accounts: AccountStore::default(),
            servers: ServerStore::default(),
            packs: PackStore::default(),
            warnings,
            snapshots: Snapshots::default(),
            settings_loaded,
            core_loaded: false,
            accounts_loaded: false,
        }
    }

    /// Load profiles, servers and resource pack state.
    pub fn load_core(&mut self) {
        let p = load_json::<ProfileStore>(&self.dirs.profiles_file());
        self.warnings.extend(p.warning);
        self.profiles = p.value;
        self.profiles.ensure_default();

        let s = load_json::<ServerStore>(&self.dirs.servers_file());
        self.warnings.extend(s.warning);
        self.servers = s.value;

        let k = load_json::<PackStore>(&self.dirs.packs_file());
        self.warnings.extend(k.warning);
        self.packs = k.value;
        self.packs.sync(&self.dirs.resource_packs_dir());
        // Only treat the stores as loaded when none of them failed with a read error. A missing
        // or corrupt file is safe (corrupt files are quarantined), but an unreadable one must not
        // be replaced by empty defaults.
        self.core_loaded = p.ok && s.ok && k.ok;
    }

    /// Load accounts; returns how many were loaded.
    pub fn load_accounts(&mut self) -> usize {
        let a = load_json::<AccountStore>(&self.dirs.accounts_file());
        self.warnings.extend(a.warning);
        self.accounts = a.value;
        self.accounts_loaded = a.ok;
        self.accounts.accounts.len()
    }

    /// How many warnings have been collected and not yet taken.
    pub fn warning_count(&self) -> usize {
        self.warnings.len()
    }

    /// Take the warnings collected so far (to show them to the user).
    pub fn take_warnings(&mut self) -> Vec<String> {
        std::mem::take(&mut self.warnings)
    }

    /// Write every store whose content changed since the last save. Returns error messages.
    pub fn save_if_changed(&mut self) -> Vec<String> {
        self.settings.sanitize();
        let mut errors = Vec::new();
        if self.settings_loaded {
            save_if_changed(
                self.dirs.settings_file(),
                &self.settings,
                &mut self.snapshots.settings,
                &mut errors,
            );
        }
        if self.core_loaded {
            save_if_changed(
                self.dirs.profiles_file(),
                &self.profiles,
                &mut self.snapshots.profiles,
                &mut errors,
            );
            save_if_changed(
                self.dirs.servers_file(),
                &self.servers,
                &mut self.snapshots.servers,
                &mut errors,
            );
            save_if_changed(
                self.dirs.packs_file(),
                &self.packs,
                &mut self.snapshots.packs,
                &mut errors,
            );
        }
        if self.accounts_loaded {
            save_if_changed(
                self.dirs.accounts_file(),
                &self.accounts,
                &mut self.snapshots.accounts,
                &mut errors,
            );
        }
        errors
    }

    /// Whether settings have been loaded and may safely be saved.
    #[cfg(test)]
    pub fn settings_loaded(&self) -> bool {
        self.settings_loaded
    }

    /// Whether accounts have been loaded and may safely be saved.
    #[cfg(test)]
    pub fn accounts_loaded(&self) -> bool {
        self.accounts_loaded
    }

    /// Whether profiles, servers and packs have been loaded and may safely be saved.
    #[cfg(test)]
    pub fn core_loaded(&self) -> bool {
        self.core_loaded
    }

    /// The selected installation (falls back to the first one).
    pub fn selected_profile(&self) -> Option<&Profile> {
        self.settings
            .selected_profile
            .as_deref()
            .and_then(|id| self.profiles.find(id))
            .or_else(|| self.profiles.profiles.first())
    }

    /// Select an installation by id.
    pub fn select_profile(&mut self, id: &str) {
        if self.profiles.find(id).is_some() {
            self.settings.selected_profile = Some(id.to_string());
        }
    }

    /// The selected account (falls back to the first one, `None` if there are none).
    pub fn selected_account(&self) -> Option<&AccountRecord> {
        self.settings
            .selected_account
            .as_deref()
            .and_then(|id| self.accounts.find(id))
            .or_else(|| self.accounts.accounts.first())
    }

    /// Select an account by id.
    pub fn select_account(&mut self, id: &str) {
        if self.accounts.find(id).is_some() {
            self.settings.selected_account = Some(id.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDir;

    fn dirs(t: &TempDir) -> LauncherDirs {
        LauncherDirs::from_root(t.path())
    }

    #[test]
    fn first_run_creates_default_profile_and_persists_it() {
        let t = TempDir::new();
        let mut data = LauncherData::load_settings(dirs(&t));
        data.load_core();
        data.load_accounts();
        assert_eq!(data.profiles.profiles.len(), 1);
        assert!(data.save_if_changed().is_empty());
        assert!(t.path().join("launcher").join("profiles.json").is_file());

        let mut again = LauncherData::load_settings(dirs(&t));
        again.load_core();
        assert_eq!(again.profiles, data.profiles);
        assert!(again.take_warnings().is_empty());
    }

    #[test]
    fn changes_are_saved_only_when_needed() {
        let t = TempDir::new();
        let mut data = LauncherData::load_settings(dirs(&t));
        data.load_core();
        data.save_if_changed();
        let settings_file = t.path().join("launcher").join("settings.json");
        let stamp = std::fs::metadata(&settings_file)
            .unwrap()
            .modified()
            .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        data.save_if_changed();
        assert_eq!(
            std::fs::metadata(&settings_file)
                .unwrap()
                .modified()
                .unwrap(),
            stamp
        );

        data.settings.animations = false;
        data.save_if_changed();
        let mut reloaded = LauncherData::load_settings(dirs(&t));
        reloaded.load_core();
        assert!(!reloaded.settings.animations);
    }

    #[test]
    fn selection_falls_back_sensibly() {
        let t = TempDir::new();
        let mut data = LauncherData::load_settings(dirs(&t));
        data.load_core();
        assert!(data.selected_account().is_none());
        let first = data.profiles.profiles[0].id.clone();
        assert_eq!(data.selected_profile().unwrap().id, first);

        let id = data.accounts.add_offline("Alex").unwrap();
        assert_eq!(data.selected_account().unwrap().id(), id);
        data.select_account("missing");
        assert_eq!(data.selected_account().unwrap().id(), id);

        let second = data.profiles.add(Profile::new("Two")).unwrap();
        data.select_profile(&second);
        assert_eq!(data.selected_profile().unwrap().id, second);
        data.profiles.delete(&second);
        assert_eq!(data.selected_profile().unwrap().id, first);
    }

    #[test]
    fn warning_count_tracks_untaken_warnings() {
        let t = TempDir::new();
        std::fs::create_dir_all(t.path().join("launcher")).unwrap();
        std::fs::write(t.path().join("launcher").join("servers.json"), "[").unwrap();
        let mut data = LauncherData::load_settings(dirs(&t));
        assert_eq!(data.warning_count(), 0);
        data.load_core();
        assert_eq!(data.warning_count(), 1);
        data.take_warnings();
        assert_eq!(data.warning_count(), 0);
    }

    #[test]
    fn corrupt_settings_produce_a_warning_and_defaults() {
        let t = TempDir::new();
        std::fs::create_dir_all(t.path().join("launcher")).unwrap();
        std::fs::write(t.path().join("launcher").join("settings.json"), "{{{").unwrap();
        let mut data = LauncherData::load_settings(dirs(&t));
        assert_eq!(data.settings, Settings::default());
        assert_eq!(data.take_warnings().len(), 1);
    }

    #[test]
    fn saving_before_loading_accounts_never_overwrites_them() {
        let t = TempDir::new();
        std::fs::create_dir_all(t.path().join("launcher")).unwrap();
        let account_file = t.path().join("launcher").join("accounts.json");
        let stored = r#"{"accounts":[{"type":"Offline","id":"a","username":"Steve","added":1}]}"#;
        std::fs::write(&account_file, stored).unwrap();

        let mut data = LauncherData::load_settings(dirs(&t));
        assert!(!data.accounts_loaded());
        data.save_if_changed();
        assert_eq!(std::fs::read_to_string(&account_file).unwrap(), stored);

        assert_eq!(data.load_accounts(), 1);
        assert!(data.accounts_loaded());
        data.accounts.remove("a");
        data.save_if_changed();
        let reloaded: AccountStore =
            serde_json::from_str(&std::fs::read_to_string(&account_file).unwrap()).unwrap();
        assert!(reloaded.accounts.is_empty());
    }

    #[test]
    fn saving_before_loading_core_never_overwrites_profiles() {
        let t = TempDir::new();
        std::fs::create_dir_all(t.path().join("launcher")).unwrap();
        let profiles_file = t.path().join("launcher").join("profiles.json");
        std::fs::write(
            &profiles_file,
            r#"{"profiles":[{"id":"p","name":"Kept","version":"1.21.11","icon":"Grass","last_played":null}],"selected":null}"#,
        )
        .unwrap();
        let before = std::fs::read_to_string(&profiles_file).unwrap();
        let mut data = LauncherData::load_settings(dirs(&t));
        assert!(!data.core_loaded());
        data.save_if_changed();
        assert_eq!(std::fs::read_to_string(&profiles_file).unwrap(), before);
    }

    #[test]
    fn unreadable_accounts_are_not_marked_loaded() {
        let t = TempDir::new();
        std::fs::create_dir_all(t.path().join("launcher").join("accounts.json")).unwrap();
        let mut data = LauncherData::load_settings(dirs(&t));
        data.load_accounts();
        assert!(!data.accounts_loaded());
    }

    #[test]
    fn unreadable_settings_are_not_saved_over() {
        let t = TempDir::new();
        let settings = t.path().join("launcher").join("settings.json");
        std::fs::create_dir_all(&settings).unwrap();
        let mut data = LauncherData::load_settings(dirs(&t));
        assert!(!data.settings_loaded());
        assert_eq!(data.take_warnings().len(), 1);
        data.save_if_changed();
        assert!(settings.is_dir());
    }
}
