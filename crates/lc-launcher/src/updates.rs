//! Launcher update check.

/// Result of an update check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateStatus {
    /// No update channel exists yet, so nothing was checked.
    NotConfigured,
}

impl UpdateStatus {
    /// Message shown on the splash screen.
    pub fn message(&self) -> &'static str {
        match self {
            Self::NotConfigured => "No update channel is configured yet, so nothing was checked",
        }
    }
}

/// Check for launcher updates.
///
/// There is no release feed to query yet, so this honestly reports
/// [`UpdateStatus::NotConfigured`] instead of pretending to contact a server.
pub fn check_for_updates() -> UpdateStatus {
    UpdateStatus::NotConfigured
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_check_is_honest_about_not_being_configured() {
        assert_eq!(check_for_updates(), UpdateStatus::NotConfigured);
        assert!(
            check_for_updates()
                .message()
                .contains("nothing was checked")
        );
    }
}
