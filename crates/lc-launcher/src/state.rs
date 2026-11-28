//! Launcher state shared by all pages: data, backend and launch orchestration.

use std::fmt;

use crate::backend::{BackendStatus, GameBackend, LaunchRequest};
use crate::model::LauncherData;
use crate::model::server::ServerAddress;
use crate::util::now_secs;

/// Why a launch did not happen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaunchBlocked {
    /// There is no installation.
    NoProfile,
    /// There is no account.
    NoAccount,
    /// The selected account cannot start a session.
    AccountCannotLaunch(String),
    /// The requested server no longer exists or its address is invalid.
    BadServer,
    /// The game backend reports it is unavailable.
    BackendUnavailable(String),
    /// The backend tried and failed.
    BackendFailed(String),
}

impl fmt::Display for LaunchBlocked {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoProfile => write!(f, "Create an installation first."),
            Self::NoAccount => write!(f, "Add an account first (an offline profile works)."),
            Self::AccountCannotLaunch(why) => write!(f, "{why}"),
            Self::BadServer => write!(f, "That server is not available."),
            Self::BackendUnavailable(why) | Self::BackendFailed(why) => write!(f, "{why}"),
        }
    }
}

/// The result of the most recent launch attempt, for display.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchLog {
    /// When it happened (Unix seconds).
    pub at: u64,
    /// Whether the game was started.
    pub success: bool,
    /// What happened.
    pub message: String,
}

/// Data plus the game backend.
pub struct LauncherState {
    /// Persisted data.
    pub data: LauncherData,
    /// The game backend.
    pub backend: Box<dyn GameBackend>,
    /// The latest launch attempt.
    pub last_launch: Option<LaunchLog>,
}

impl LauncherState {
    /// Create the state.
    pub fn new(data: LauncherData, backend: Box<dyn GameBackend>) -> Self {
        Self {
            data,
            backend,
            last_launch: None,
        }
    }

    /// Current backend status.
    pub fn backend_status(&self) -> BackendStatus {
        self.backend.status()
    }

    /// Launch the selected installation with the selected account, optionally into a server.
    ///
    /// When the backend is ready this calls it immediately, with no extra steps.
    pub fn launch(&mut self, server_id: Option<&str>) -> Result<(), LaunchBlocked> {
        let result = self.try_launch(server_id);
        self.last_launch = Some(match &result {
            Ok(()) => LaunchLog {
                at: now_secs(),
                success: true,
                message: "The game was started.".to_string(),
            },
            Err(e) => LaunchLog {
                at: now_secs(),
                success: false,
                message: e.to_string(),
            },
        });
        result
    }

    fn try_launch(&mut self, server_id: Option<&str>) -> Result<(), LaunchBlocked> {
        let profile = self
            .data
            .selected_profile()
            .cloned()
            .ok_or(LaunchBlocked::NoProfile)?;
        let account = self
            .data
            .selected_account()
            .cloned()
            .ok_or(LaunchBlocked::NoAccount)?;
        if !account.can_launch() {
            return Err(LaunchBlocked::AccountCannotLaunch(format!(
                "{}: {}.",
                account.username(),
                account.status_label()
            )));
        }
        let server = match server_id {
            Some(id) => {
                let entry = self.data.servers.find(id).ok_or(LaunchBlocked::BadServer)?;
                Some(ServerAddress::parse(&entry.address).map_err(|_| LaunchBlocked::BadServer)?)
            }
            None => None,
        };
        if let BackendStatus::Unavailable(reason) = self.backend.status() {
            return Err(LaunchBlocked::BackendUnavailable(reason));
        }
        let game_dir = profile.resolved_game_dir(&self.data.dirs);
        let _ = std::fs::create_dir_all(&game_dir);
        let request = LaunchRequest {
            profile: profile.clone(),
            account: account.net_kind(),
            server,
            game_dir,
        };
        self.backend
            .launch(&request)
            .map_err(|e| LaunchBlocked::BackendFailed(e.0))?;

        let now = now_secs();
        if let Some(p) = self.data.profiles.find_mut(&profile.id) {
            p.last_played = Some(now);
        }
        if let Some(id) = server_id {
            self.data.servers.mark_played(id);
        }
        Ok(())
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::backend::LaunchFailure;
    use crate::model::server::ServerEntry;
    use crate::paths::LauncherDirs;
    use crate::test_support::TempDir;
    use std::cell::RefCell;
    use std::rc::Rc;

    /// A backend that records launches, for tests.
    pub(crate) struct MockBackend {
        pub ready: Rc<RefCell<bool>>,
        pub launches: Rc<RefCell<Vec<LaunchRequest>>>,
        pub fail: bool,
    }

    impl GameBackend for MockBackend {
        fn status(&self) -> BackendStatus {
            if *self.ready.borrow() {
                BackendStatus::Ready
            } else {
                BackendStatus::Unavailable("not yet".into())
            }
        }
        fn launch(&mut self, request: &LaunchRequest) -> Result<(), LaunchFailure> {
            if self.fail {
                return Err(LaunchFailure("boom".into()));
            }
            self.launches.borrow_mut().push(request.clone());
            Ok(())
        }
    }

    pub(crate) fn test_state(t: &TempDir, backend: Box<dyn GameBackend>) -> LauncherState {
        let mut data = LauncherData::load_settings(LauncherDirs::from_root(t.path()));
        data.load_core();
        data.load_accounts();
        LauncherState::new(data, backend)
    }

    #[test]
    fn launch_requires_account() {
        let t = TempDir::new();
        let mut state = test_state(
            &t,
            Box::new(MockBackend {
                ready: Rc::new(RefCell::new(true)),
                launches: Rc::new(RefCell::new(Vec::new())),
                fail: false,
            }),
        );
        assert_eq!(state.launch(None), Err(LaunchBlocked::NoAccount));
        assert!(!state.last_launch.as_ref().unwrap().success);
    }

    #[test]
    fn unavailable_backend_blocks_launch_honestly() {
        let t = TempDir::new();
        let mut state = test_state(
            &t,
            Box::new(MockBackend {
                ready: Rc::new(RefCell::new(false)),
                launches: Rc::new(RefCell::new(Vec::new())),
                fail: false,
            }),
        );
        state.data.accounts.add_offline("Alex").unwrap();
        let err = state.launch(None).unwrap_err();
        assert!(matches!(err, LaunchBlocked::BackendUnavailable(_)));
        assert!(state.data.selected_profile().unwrap().last_played.is_none());
    }

    #[test]
    fn launch_happens_immediately_once_backend_becomes_ready() {
        let t = TempDir::new();
        let ready = Rc::new(RefCell::new(false));
        let launches = Rc::new(RefCell::new(Vec::new()));
        let mut state = test_state(
            &t,
            Box::new(MockBackend {
                ready: ready.clone(),
                launches: launches.clone(),
                fail: false,
            }),
        );
        state.data.accounts.add_offline("Alex").unwrap();
        assert!(state.launch(None).is_err());
        assert!(launches.borrow().is_empty());

        *ready.borrow_mut() = true;
        state.launch(None).unwrap();
        assert_eq!(launches.borrow().len(), 1);
        assert_eq!(
            launches.borrow()[0].account,
            lc_net::AccountKind::Offline {
                username: "Alex".into()
            }
        );
        assert!(state.data.selected_profile().unwrap().last_played.is_some());
        assert!(state.last_launch.as_ref().unwrap().success);
    }

    #[test]
    fn joining_a_server_records_it_as_recent() {
        let t = TempDir::new();
        let ready = Rc::new(RefCell::new(true));
        let launches = Rc::new(RefCell::new(Vec::new()));
        let mut state = test_state(
            &t,
            Box::new(MockBackend {
                ready,
                launches: launches.clone(),
                fail: false,
            }),
        );
        state.data.accounts.add_offline("Alex").unwrap();
        let id = state
            .data
            .servers
            .add(ServerEntry::new("S", "play.example.org:25570", ""))
            .unwrap();
        state.launch(Some(&id)).unwrap();
        let req = launches.borrow()[0].clone();
        assert_eq!(req.server.unwrap().port, 25570);
        assert_eq!(state.data.servers.recent(5).len(), 1);
        assert_eq!(state.launch(Some("nope")), Err(LaunchBlocked::BadServer));
    }

    #[test]
    fn backend_failure_is_reported_and_not_recorded_as_played() {
        let t = TempDir::new();
        let mut state = test_state(
            &t,
            Box::new(MockBackend {
                ready: Rc::new(RefCell::new(true)),
                launches: Rc::new(RefCell::new(Vec::new())),
                fail: true,
            }),
        );
        state.data.accounts.add_offline("Alex").unwrap();
        assert_eq!(
            state.launch(None),
            Err(LaunchBlocked::BackendFailed("boom".into()))
        );
        assert!(state.data.selected_profile().unwrap().last_played.is_none());
    }

    #[test]
    fn microsoft_accounts_cannot_launch_yet() {
        let t = TempDir::new();
        let mut state = test_state(
            &t,
            Box::new(MockBackend {
                ready: Rc::new(RefCell::new(true)),
                launches: Rc::new(RefCell::new(Vec::new())),
                fail: false,
            }),
        );
        state
            .data
            .accounts
            .accounts
            .push(crate::model::account::AccountRecord::Microsoft {
                id: "m".into(),
                username: "Notch".into(),
                added: 0,
            });
        assert!(matches!(
            state.launch(None),
            Err(LaunchBlocked::AccountCannotLaunch(_))
        ));
    }
}
