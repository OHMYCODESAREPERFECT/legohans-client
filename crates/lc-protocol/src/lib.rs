//! Minecraft Java Edition protocol: packet definitions, codecs and connection state machine.

/// The Minecraft Java Edition version this client targets.
pub const TARGET_GAME_VERSION: &str = "1.21.11";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn targets_expected_version() {
        assert_eq!(TARGET_GAME_VERSION, "1.21.11");
    }
}
