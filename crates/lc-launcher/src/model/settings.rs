//! Launcher settings and the searchable settings catalogue.

use serde::{Deserialize, Serialize};

/// Accent color preset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Accent {
    /// Vivid violet (default).
    #[default]
    Violet,
    /// Pink-leaning amethyst.
    Amethyst,
    /// Deep indigo-blue.
    Indigo,
    /// Soft orchid.
    Orchid,
}

impl Accent {
    /// All presets in display order.
    pub const ALL: [Accent; 4] = [Self::Violet, Self::Amethyst, Self::Indigo, Self::Orchid];

    /// Display name.
    pub fn name(self) -> &'static str {
        match self {
            Self::Violet => "Violet",
            Self::Amethyst => "Amethyst",
            Self::Indigo => "Indigo",
            Self::Orchid => "Orchid",
        }
    }
}

/// What the launcher does once a game launch succeeds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum AfterLaunch {
    /// Leave the launcher open.
    #[default]
    KeepOpen,
    /// Minimize the launcher.
    Minimize,
    /// Close the launcher.
    Close,
}

impl AfterLaunch {
    /// All choices in display order.
    pub const ALL: [AfterLaunch; 3] = [Self::KeepOpen, Self::Minimize, Self::Close];

    /// Display name.
    pub fn name(self) -> &'static str {
        match self {
            Self::KeepOpen => "Keep the launcher open",
            Self::Minimize => "Minimize the launcher",
            Self::Close => "Close the launcher",
        }
    }
}

/// Where the Home page artwork comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum HomeBackground {
    /// Use the panorama from the user's own Minecraft install when found, else generated art.
    #[default]
    Auto,
    /// Always use the generated landscape.
    Generated,
}

impl HomeBackground {
    /// All choices in display order.
    pub const ALL: [HomeBackground; 2] = [Self::Auto, Self::Generated];

    /// Display name.
    pub fn name(self) -> &'static str {
        match self {
            Self::Auto => "Minecraft panorama if available",
            Self::Generated => "Always generated landscape",
        }
    }
}

/// Persisted launcher settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Accent color preset.
    pub accent: Accent,
    /// UI zoom factor.
    pub ui_scale: f32,
    /// Whether animations and transitions run.
    pub animations: bool,
    /// Always use the narrow icon-only sidebar.
    pub compact_sidebar: bool,
    /// Show the splash screen at startup.
    pub show_splash: bool,
    /// Run the update check at startup.
    pub check_updates: bool,
    /// Behaviour after launching the game.
    pub after_launch: AfterLaunch,
    /// Home artwork source.
    pub home_background: HomeBackground,
    /// Optional override of the Minecraft folder used to find the panorama.
    pub minecraft_dir: String,
    /// Selected installation id.
    pub selected_profile: Option<String>,
    /// Selected account id.
    pub selected_account: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            accent: Accent::default(),
            ui_scale: 1.0,
            animations: true,
            compact_sidebar: false,
            show_splash: true,
            check_updates: true,
            after_launch: AfterLaunch::default(),
            home_background: HomeBackground::default(),
            minecraft_dir: String::new(),
            selected_profile: None,
            selected_account: None,
        }
    }
}

/// Smallest and largest allowed UI scale.
pub const UI_SCALE_RANGE: std::ops::RangeInclusive<f32> = 0.75..=1.5;

impl Settings {
    /// Clamp out-of-range values (for example from a hand-edited file).
    pub fn sanitize(&mut self) {
        if !self.ui_scale.is_finite() {
            self.ui_scale = 1.0;
        }
        self.ui_scale = self
            .ui_scale
            .clamp(*UI_SCALE_RANGE.start(), *UI_SCALE_RANGE.end());
    }
}

/// Settings categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    /// Look and feel.
    Appearance,
    /// Launcher behaviour.
    Launcher,
    /// Game-related options.
    Game,
    /// Folders on disk.
    Storage,
    /// About and maintenance.
    About,
}

impl Category {
    /// All categories in display order.
    pub const ALL: [Category; 5] = [
        Self::Appearance,
        Self::Launcher,
        Self::Game,
        Self::Storage,
        Self::About,
    ];

    /// Display title.
    pub fn title(self) -> &'static str {
        match self {
            Self::Appearance => "Appearance",
            Self::Launcher => "Launcher",
            Self::Game => "Game",
            Self::Storage => "Storage",
            Self::About => "About",
        }
    }
}

/// Identifier of each individual setting control.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingId {
    /// Accent preset.
    Accent,
    /// UI scale slider.
    UiScale,
    /// Animations toggle.
    Animations,
    /// Compact sidebar toggle.
    CompactSidebar,
    /// Home artwork source.
    HomeBackground,
    /// Splash screen toggle.
    ShowSplash,
    /// Startup update check toggle.
    CheckUpdates,
    /// Behaviour after launch.
    AfterLaunch,
    /// Minecraft folder override.
    MinecraftDir,
    /// Launcher data folder (read-only).
    DataFolder,
    /// Screenshots folder (read-only).
    ScreenshotsFolder,
    /// Reset settings button.
    Reset,
    /// Version information.
    About,
}

/// Static description of a setting, used for rendering and search.
#[derive(Debug, Clone, Copy)]
pub struct SettingMeta {
    /// Which control this is.
    pub id: SettingId,
    /// The category it belongs to.
    pub category: Category,
    /// Short title.
    pub title: &'static str,
    /// One-line explanation.
    pub description: &'static str,
    /// Extra search terms.
    pub keywords: &'static [&'static str],
}

/// Every setting, in display order.
pub const SETTINGS: &[SettingMeta] = &[
    SettingMeta {
        id: SettingId::Accent,
        category: Category::Appearance,
        title: "Accent color",
        description: "Highlight color used for buttons, glow and selection.",
        keywords: &["theme", "colour", "purple", "violet"],
    },
    SettingMeta {
        id: SettingId::UiScale,
        category: Category::Appearance,
        title: "Interface scale",
        description: "Make the whole launcher larger or smaller.",
        keywords: &["zoom", "size", "dpi", "text"],
    },
    SettingMeta {
        id: SettingId::Animations,
        category: Category::Appearance,
        title: "Animations",
        description: "Page transitions, glow and moving backgrounds.",
        keywords: &["motion", "transitions", "reduce", "effects"],
    },
    SettingMeta {
        id: SettingId::CompactSidebar,
        category: Category::Appearance,
        title: "Compact sidebar",
        description: "Always show the narrow icon-only sidebar.",
        keywords: &["navigation", "menu", "narrow"],
    },
    SettingMeta {
        id: SettingId::HomeBackground,
        category: Category::Appearance,
        title: "Home artwork",
        description: "Use your own Minecraft panorama when found, or the generated landscape.",
        keywords: &["background", "panorama", "wallpaper", "landscape"],
    },
    SettingMeta {
        id: SettingId::ShowSplash,
        category: Category::Launcher,
        title: "Splash screen",
        description: "Show the animated splash screen while starting.",
        keywords: &["startup", "intro", "loading"],
    },
    SettingMeta {
        id: SettingId::CheckUpdates,
        category: Category::Launcher,
        title: "Check for updates on startup",
        description: "Currently has no effect: no update channel exists yet.",
        keywords: &["update", "version", "upgrade"],
    },
    SettingMeta {
        id: SettingId::AfterLaunch,
        category: Category::Launcher,
        title: "After launching the game",
        description: "What the launcher does once the game has started.",
        keywords: &["close", "minimize", "hide", "keep open"],
    },
    SettingMeta {
        id: SettingId::MinecraftDir,
        category: Category::Game,
        title: "Minecraft folder",
        description: "Your official .minecraft folder, used only to find the vanilla panorama.",
        keywords: &["path", "directory", "install", "assets"],
    },
    SettingMeta {
        id: SettingId::DataFolder,
        category: Category::Storage,
        title: "Launcher data folder",
        description: "Where profiles, accounts, servers and packs are stored.",
        keywords: &["path", "directory", "files", "config"],
    },
    SettingMeta {
        id: SettingId::ScreenshotsFolder,
        category: Category::Storage,
        title: "Screenshots folder",
        description: "Where launcher-wide screenshots are kept.",
        keywords: &["path", "directory", "images", "pictures"],
    },
    SettingMeta {
        id: SettingId::Reset,
        category: Category::About,
        title: "Reset settings",
        description: "Restore all launcher settings to their defaults. Profiles are kept.",
        keywords: &["default", "restore", "clear"],
    },
    SettingMeta {
        id: SettingId::About,
        category: Category::About,
        title: "About Legohan's Client",
        description: "Version and target game version.",
        keywords: &["info", "build", "license"],
    },
];

/// Return the settings matching `query`. Every whitespace-separated term must match the
/// title, description, category or a keyword (case-insensitive). An empty query matches all.
pub fn search(query: &str) -> Vec<&'static SettingMeta> {
    let terms: Vec<String> = query.split_whitespace().map(|t| t.to_lowercase()).collect();
    SETTINGS
        .iter()
        .filter(|meta| {
            let haystack = format!(
                "{} {} {} {}",
                meta.title,
                meta.description,
                meta.category.title(),
                meta.keywords.join(" ")
            )
            .to_lowercase();
            terms.iter().all(|t| haystack.contains(t))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_sane() {
        let s = Settings::default();
        assert!(s.animations && s.show_splash && s.check_updates);
        assert_eq!(s.ui_scale, 1.0);
    }

    #[test]
    fn sanitize_clamps_and_repairs() {
        let mut s = Settings {
            ui_scale: 9.0,
            ..Settings::default()
        };
        s.sanitize();
        assert_eq!(s.ui_scale, 1.5);
        s.ui_scale = f32::NAN;
        s.sanitize();
        assert_eq!(s.ui_scale, 1.0);
    }

    #[test]
    fn empty_query_returns_everything() {
        assert_eq!(search("").len(), SETTINGS.len());
        assert_eq!(search("   ").len(), SETTINGS.len());
    }

    #[test]
    fn search_matches_title_keywords_and_category() {
        assert!(search("accent").iter().any(|m| m.id == SettingId::Accent));
        assert!(search("zoom").iter().any(|m| m.id == SettingId::UiScale));
        assert!(
            search("STORAGE")
                .iter()
                .any(|m| m.id == SettingId::DataFolder)
        );
    }

    #[test]
    fn all_terms_must_match() {
        assert!(search("accent folder").is_empty());
        assert!(search("nonexistent-setting").is_empty());
    }

    #[test]
    fn every_setting_has_a_unique_id() {
        for (i, a) in SETTINGS.iter().enumerate() {
            assert!(SETTINGS.iter().skip(i + 1).all(|b| a.id != b.id));
        }
    }

    #[test]
    fn partial_json_loads_with_defaults() {
        let s: Settings = serde_json::from_str(r#"{"animations":false}"#).unwrap();
        assert!(!s.animations);
        assert!(s.show_splash);
    }
}
