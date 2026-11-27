//! Minecraft Java Edition protocol: packet definitions, codecs and connection state machine.
//!
//! Only the pieces needed so far exist: the primitive codecs and the
//! handshake/status ("server list ping") packets.

pub mod codec;
pub mod status;

/// The Minecraft Java Edition version this client targets.
pub const TARGET_GAME_VERSION: &str = "1.21.11";

/// Protocol version number of [`TARGET_GAME_VERSION`] (774 for 1.21.11, per the Minecraft Wiki).
pub const PROTOCOL_VERSION: i32 = 774;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn targets_expected_version() {
        assert_eq!(TARGET_GAME_VERSION, "1.21.11");
        assert_eq!(PROTOCOL_VERSION, 774);
    }
}
