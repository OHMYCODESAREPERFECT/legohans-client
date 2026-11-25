//! Input handling: keybinds, camera, movement and physics prediction.

/// Placeholder marker until input handling exists.
pub const BACKEND: &str = "none";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_backend_yet() {
        assert_eq!(BACKEND, "none");
    }
}
