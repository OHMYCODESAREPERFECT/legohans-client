//! The boundary between the launcher and the game itself.

use std::path::PathBuf;

use lc_net::AccountKind;

use crate::model::profile::Profile;
use crate::model::server::ServerAddress;

/// Everything the game needs to start.
#[derive(Debug, Clone, PartialEq)]
pub struct LaunchRequest {
    /// The installation to start.
    pub profile: Profile,
    /// The account kind to authenticate with.
    pub account: AccountKind,
    /// A server to join immediately, if any.
    pub server: Option<ServerAddress>,
    /// The game directory.
    pub game_dir: PathBuf,
}

/// Whether the game can be launched right now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendStatus {
    /// The game cannot be launched, with the reason.
    Unavailable(String),
    /// The game can be launched immediately.
    #[allow(
        dead_code,
        reason = "returned by the game backend once the Rust client exists"
    )]
    Ready,
}

/// A failed launch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchFailure(pub String);

/// The game implementation the launcher starts.
///
/// The Play button calls [`GameBackend::launch`] directly when [`GameBackend::status`] is
/// [`BackendStatus::Ready`], so there are no intermediate steps once a backend exists.
pub trait GameBackend {
    /// Whether launching is possible right now.
    fn status(&self) -> BackendStatus;
    /// Start the game.
    fn launch(&mut self, request: &LaunchRequest) -> Result<(), LaunchFailure>;
}

/// The backend used today: the Rust game client does not exist yet, so it reports itself
/// unavailable and never launches anything.
#[derive(Debug, Default)]
pub struct NativeClientBackend;

impl GameBackend for NativeClientBackend {
    fn status(&self) -> BackendStatus {
        BackendStatus::Unavailable(
            "The Rust game client (world, rendering and input) is not implemented yet.".to_string(),
        )
    }

    fn launch(&mut self, _request: &LaunchRequest) -> Result<(), LaunchFailure> {
        Err(LaunchFailure(
            "The Rust game client is not implemented yet, so nothing was launched.".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_backend_is_honestly_unavailable() {
        let mut backend = NativeClientBackend;
        assert!(matches!(backend.status(), BackendStatus::Unavailable(_)));
        let request = LaunchRequest {
            profile: Profile::new("x"),
            account: AccountKind::Offline {
                username: "Alex".into(),
            },
            server: None,
            game_dir: PathBuf::from("."),
        };
        assert!(backend.launch(&request).is_err());
    }
}
