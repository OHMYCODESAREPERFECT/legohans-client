//! The launcher user interface.

pub mod artwork;
pub mod icons;
pub mod pages;
pub mod sidebar;
pub mod splash;
pub mod theme;
pub mod toasts;
pub mod widgets;

/// Sidebar pages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Page {
    /// Landing dashboard.
    #[default]
    Home,
    /// Launch controls.
    Play,
    /// Installation profiles.
    Installations,
    /// Resource packs.
    ResourcePacks,
    /// Saved servers.
    Servers,
    /// Accounts.
    Accounts,
    /// Skin, cape and elytra editor.
    SkinEditor,
    /// Settings.
    Settings,
    /// Screenshots.
    Screenshots,
}

impl Page {
    /// All pages in sidebar order.
    pub const ALL: [Page; 9] = [
        Self::Home,
        Self::Play,
        Self::Installations,
        Self::ResourcePacks,
        Self::Servers,
        Self::Accounts,
        Self::SkinEditor,
        Self::Settings,
        Self::Screenshots,
    ];

    /// Sidebar and page title.
    pub fn title(self) -> &'static str {
        match self {
            Self::Home => "Home",
            Self::Play => "Play",
            Self::Installations => "Installations",
            Self::ResourcePacks => "Resource Packs",
            Self::Servers => "Servers",
            Self::Accounts => "Accounts",
            Self::SkinEditor => "Skin Editor",
            Self::Settings => "Settings",
            Self::Screenshots => "Screenshots",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_pages_have_unique_titles() {
        assert_eq!(Page::ALL.len(), 9);
        let mut titles: Vec<_> = Page::ALL.iter().map(|p| p.title()).collect();
        titles.sort();
        titles.dedup();
        assert_eq!(titles.len(), 9);
    }
}
