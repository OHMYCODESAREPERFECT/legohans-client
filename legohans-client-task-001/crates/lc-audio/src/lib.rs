//! Audio engine: sound playback and mixing.

/// Placeholder marker until the audio engine exists.
pub const BACKEND: &str = "none";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_backend_yet() {
        assert_eq!(BACKEND, "none");
    }
}
