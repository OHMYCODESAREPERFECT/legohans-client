//! User interface: menus, HUD and inventory screens.

/// Visual-only in-game menu system, separate from the launcher UI.
pub mod ingame;

/// Placeholder marker for the not-yet-implemented gameplay UI backend.
pub const BACKEND: &str = "none";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_backend_yet() {
        assert_eq!(BACKEND, "none");
    }
}
