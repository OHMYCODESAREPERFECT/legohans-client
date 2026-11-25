//! Networking: TCP connection, compression, encryption and account authentication.

/// The kind of account a session is started with.
///
/// Both kinds are first-class from the start. Neither weakens server security:
/// - [`AccountKind::Microsoft`] is a licensed account authenticated through Microsoft/Mojang services.
/// - [`AccountKind::Offline`] is a local profile that is only valid on servers that
///   themselves allow offline-mode (`online-mode=false`). It is never a way around a
///   server's authentication.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountKind {
    /// A licensed Microsoft account (online mode).
    Microsoft,
    /// A local, unauthenticated profile for offline-mode servers and singleplayer-style testing.
    Offline {
        /// The username shown in game.
        username: String,
    },
}

impl AccountKind {
    /// Whether this account requires a successful Microsoft/Mojang login before connecting.
    pub fn requires_online_auth(&self) -> bool {
        matches!(self, Self::Microsoft)
    }
}

/// The game version the protocol layer targets.
pub const GAME_VERSION: &str = lc_protocol::TARGET_GAME_VERSION;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn microsoft_requires_online_auth() {
        assert!(AccountKind::Microsoft.requires_online_auth());
    }

    #[test]
    fn offline_does_not_require_online_auth() {
        let offline = AccountKind::Offline {
            username: "Player".into(),
        };
        assert!(!offline.requires_online_auth());
    }

    #[test]
    fn uses_protocol_game_version() {
        assert_eq!(GAME_VERSION, "1.21.11");
    }
}
