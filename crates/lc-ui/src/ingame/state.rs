//! Visual-only menu state, navigation stack and placeholder data.

/// Every screen in the in-game UI preview.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Screen {
    /// Mojang-style loading screen.
    Loading,
    /// Main menu with animated panorama.
    MainMenu,
    /// Singleplayer world selection.
    Singleplayer,
    /// Multiplayer server selection.
    Multiplayer,
    /// Create New World screen.
    CreateWorld,
    /// World settings screen.
    WorldSettings,
    /// Main options screen.
    Options,
    /// Video settings.
    Video,
    /// Controls screen.
    Controls,
    /// Language selection.
    Language,
    /// Accessibility settings.
    Accessibility,
    /// Pause menu.
    Pause,
}

impl Screen {
    /// Stable command-line name used by the screenshot helper.
    pub fn arg_name(self) -> &'static str {
        match self {
            Self::Loading => "loading",
            Self::MainMenu => "main-menu",
            Self::Singleplayer => "singleplayer",
            Self::Multiplayer => "multiplayer",
            Self::CreateWorld => "create-world",
            Self::WorldSettings => "world-settings",
            Self::Options => "options",
            Self::Video => "video",
            Self::Controls => "controls",
            Self::Language => "language",
            Self::Accessibility => "accessibility",
            Self::Pause => "pause",
        }
    }

    /// Header shown at the top of each screen.
    pub fn title(self) -> &'static str {
        match self {
            Self::Loading => "Loading",
            Self::MainMenu => "",
            Self::Singleplayer => "Select World",
            Self::Multiplayer => "Multiplayer",
            Self::CreateWorld => "Create New World",
            Self::WorldSettings => "World Settings",
            Self::Options => "Options",
            Self::Video => "Video Settings",
            Self::Controls => "Controls",
            Self::Language => "Language",
            Self::Accessibility => "Accessibility Settings",
            Self::Pause => "Game Paused",
        }
    }

    /// Parse a command-line screen name.
    pub fn from_arg(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|screen| screen.arg_name() == value)
    }

    /// All screens, in a stable order used by tests and capture tooling.
    pub const ALL: [Self; 12] = [
        Self::Loading,
        Self::MainMenu,
        Self::Singleplayer,
        Self::Multiplayer,
        Self::CreateWorld,
        Self::WorldSettings,
        Self::Options,
        Self::Video,
        Self::Controls,
        Self::Language,
        Self::Accessibility,
        Self::Pause,
    ];
}

/// A fake singleplayer world entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldEntry {
    /// Display name.
    pub name: &'static str,
    /// Secondary information line.
    pub detail: &'static str,
    /// Icon variant.
    pub icon: usize,
}

/// A fake multiplayer server entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerEntry {
    /// Display name.
    pub name: &'static str,
    /// Address shown under the name.
    pub address: &'static str,
    /// Message of the day.
    pub motd: &'static str,
    /// Ping bar index, 1..=5, or 0 for unreachable.
    pub ping: usize,
    /// Player count text.
    pub players: &'static str,
}

/// Placeholder language entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LanguageEntry {
    /// Language code.
    pub code: &'static str,
    /// Display name.
    pub name: &'static str,
}

/// Placeholder worlds shown in the visual list.
pub const WORLDS: [WorldEntry; 5] = [
    WorldEntry {
        name: "Mountain Hollow",
        detail: "Overworld • 4 days • Survival",
        icon: 0,
    },
    WorldEntry {
        name: "Copper Coast",
        detail: "Overworld • 18 days • Creative",
        icon: 1,
    },
    WorldEntry {
        name: "Deep Dark Test",
        detail: "Overworld • 2 days • Hardcore",
        icon: 2,
    },
    WorldEntry {
        name: "Pale Garden",
        detail: "Overworld • 31 days • Survival",
        icon: 3,
    },
    WorldEntry {
        name: "Old World 1.12",
        detail: "Upgrade needed • 96 days",
        icon: 4,
    },
];

/// Placeholder servers shown in the visual list.
pub const SERVERS: [ServerEntry; 4] = [
    ServerEntry {
        name: "Hermitcraft",
        address: "mc.hermitcraft.com",
        motd: "A friendly survival community",
        ping: 5,
        players: "32/150",
    },
    ServerEntry {
        name: "Legohan Test Realm",
        address: "play.example.net",
        motd: "Visual preview only - no connection is made",
        ping: 4,
        players: "7/20",
    },
    ServerEntry {
        name: "Block Party",
        address: "minigames.example.org",
        motd: "Bed Wars • SkyWars • Spleef",
        ping: 3,
        players: "118/500",
    },
    ServerEntry {
        name: "Offline Archive",
        address: "old.example.com",
        motd: "Connection refused",
        ping: 0,
        players: "0/0",
    },
];

/// Placeholder languages.
pub const LANGUAGES: [LanguageEntry; 8] = [
    LanguageEntry {
        code: "en_us",
        name: "English (United States)",
    },
    LanguageEntry {
        code: "en_gb",
        name: "English (United Kingdom)",
    },
    LanguageEntry {
        code: "de_de",
        name: "Deutsch (Deutschland)",
    },
    LanguageEntry {
        code: "es_es",
        name: "Español (España)",
    },
    LanguageEntry {
        code: "fr_fr",
        name: "Français (France)",
    },
    LanguageEntry {
        code: "ja_jp",
        name: "日本語 (日本)",
    },
    LanguageEntry {
        code: "pt_br",
        name: "Português (Brasil)",
    },
    LanguageEntry {
        code: "zh_cn",
        name: "简体中文 (中国)",
    },
];

/// Mutable visual-only settings. Nothing here is saved or applied to gameplay.
#[derive(Debug, Clone, PartialEq)]
pub struct VisualSettings {
    /// Graphics quality label index.
    pub graphics: u8,
    /// Render distance, 2..=32.
    pub render_distance: u8,
    /// Smooth lighting amount, 0..=1.
    pub smooth_lighting: f32,
    /// GUI scale label index.
    pub gui_scale: u8,
    /// Frame-rate limit label index.
    pub fps_limit: u8,
    /// Vertical sync toggle.
    pub vsync: bool,
    /// Fullscreen toggle.
    pub fullscreen: bool,
    /// Panorama scroll speed, 0..=2.
    pub panorama_speed: f32,
    /// High-contrast text toggle.
    pub high_contrast: bool,
    /// Reduced-motion toggle.
    pub reduced_motion: bool,
    /// Text background toggle.
    pub text_background: bool,
    /// World difficulty label index.
    pub difficulty: u8,
    /// Allow cheats toggle for the fake world.
    pub allow_cheats: bool,
    /// World type label index.
    pub world_type: u8,
}

impl Default for VisualSettings {
    fn default() -> Self {
        Self {
            graphics: 2,
            render_distance: 12,
            smooth_lighting: 1.0,
            gui_scale: 1,
            fps_limit: 2,
            vsync: true,
            fullscreen: false,
            panorama_speed: 1.0,
            high_contrast: false,
            reduced_motion: false,
            text_background: true,
            difficulty: 2,
            allow_cheats: false,
            world_type: 0,
        }
    }
}

/// Complete state for the visual-only in-game menu shell.
#[derive(Debug, Clone)]
pub struct MenuState {
    /// Current screen.
    pub screen: Screen,
    /// Screens to return to with Escape or Back.
    stack: Vec<Screen>,
    /// Elapsed preview time in seconds.
    pub time: f32,
    /// Screen fade-in progress, 0..=1.
    pub transition: f32,
    /// Selected fake world.
    pub selected_world: usize,
    /// Selected fake server.
    pub selected_server: usize,
    /// Selected language.
    pub selected_language: usize,
    /// Fake world name field.
    pub world_name: String,
    /// Fake seed field.
    pub seed: String,
    /// Mutable visual settings.
    pub settings: VisualSettings,
    /// Optional fixed panorama yaw for deterministic captures.
    pub yaw_override: Option<f32>,
    /// Optional fixed panorama pitch for deterministic captures.
    pub pitch_override: Option<f32>,
}

impl MenuState {
    /// Create state, optionally starting on a specific screen.
    pub fn new(screen: Screen) -> Self {
        Self {
            screen,
            stack: Vec::new(),
            time: 0.0,
            transition: 0.0,
            selected_world: 0,
            selected_server: 0,
            selected_language: 0,
            world_name: "New World".to_string(),
            seed: String::new(),
            settings: VisualSettings::default(),
            yaw_override: None,
            pitch_override: None,
        }
    }

    /// Advance animations.
    pub fn tick(&mut self, dt: f32) {
        self.time += dt.max(0.0);
        let speed = if self.settings.reduced_motion {
            3.5
        } else {
            3.0
        };
        self.transition = (self.transition + dt * speed).clamp(0.0, 1.0);
    }

    /// Current panorama yaw.
    pub fn yaw(&self) -> f32 {
        self.yaw_override
            .unwrap_or(self.time * 0.025 * self.settings.panorama_speed)
    }

    /// Current panorama pitch.
    pub fn pitch(&self) -> f32 {
        self.pitch_override.unwrap_or(0.0)
    }

    /// Navigate to `screen`, remembering where to return.
    pub fn go(&mut self, screen: Screen) {
        if screen != self.screen {
            self.stack.push(self.screen);
            self.screen = screen;
            self.transition = 0.0;
        }
    }

    /// Return to the previous screen, or the main menu at the root.
    pub fn back(&mut self) {
        self.screen = self.stack.pop().unwrap_or(Screen::MainMenu);
        self.transition = 0.0;
    }

    /// Return directly to the main menu, discarding the navigation history.
    pub fn reset_stack(&mut self) {
        self.stack.clear();
    }

    /// Whether a Back button can return somewhere meaningful.
    pub fn can_go_back(&self) -> bool {
        !self.stack.is_empty() || self.screen != Screen::MainMenu
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_screens_have_unique_arg_names() {
        let mut names = std::collections::HashSet::new();
        for screen in Screen::ALL {
            assert!(names.insert(screen.arg_name()));
            assert_eq!(Screen::from_arg(screen.arg_name()), Some(screen));
        }
        assert_eq!(names.len(), Screen::ALL.len());
    }

    #[test]
    fn navigation_returns_to_the_previous_screen() {
        let mut state = MenuState::new(Screen::MainMenu);
        state.go(Screen::Options);
        state.go(Screen::Video);
        assert_eq!(state.screen, Screen::Video);
        state.back();
        assert_eq!(state.screen, Screen::Options);
        state.back();
        assert_eq!(state.screen, Screen::MainMenu);
    }

    #[test]
    fn placeholders_are_non_empty_and_consistent() {
        assert!(WORLDS.iter().all(|w| !w.name.is_empty()));
        assert!(SERVERS.iter().all(|s| !s.address.is_empty()));
        assert!(LANGUAGES.iter().all(|l| !l.name.is_empty()));
        assert_eq!(WORLDS.len(), 5);
        assert_eq!(SERVERS.len(), 4);
        assert_eq!(LANGUAGES.len(), 8);
    }
}
