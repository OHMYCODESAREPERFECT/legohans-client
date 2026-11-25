//! User interface: menus, HUD and inventory screens.

/// Placeholder marker until the UI exists.
pub const BACKEND: &str = "none";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_backend_yet() {
        assert_eq!(BACKEND, "none");
    }
}
