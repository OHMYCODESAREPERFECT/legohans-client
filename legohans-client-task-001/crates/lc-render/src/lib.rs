//! Rendering: chunk meshing, texture atlas and GPU backend.

/// Placeholder marker until the renderer exists (Task 00x).
pub const BACKEND: &str = "none";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_backend_yet() {
        assert_eq!(BACKEND, "none");
    }
}
