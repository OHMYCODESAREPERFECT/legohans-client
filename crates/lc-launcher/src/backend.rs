//! The boundary between the launcher and the game itself.
//!
//! The playable entry point today is the visual-only game-client window implemented by
//! `lc_ui::ingame`. The launcher starts it through the same `--ingame-ui` command-line path that
//! the binaries already expose, so no second rendering engine is introduced.

use std::{
    env,
    ffi::OsStr,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

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
    #[allow(
        dead_code,
        reason = "alternate and test backends still report unavailable statuses"
    )]
    Unavailable(String),
    /// The game can be launched immediately.
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

/// The command used to start the visual-only game-client window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisualClientCommand {
    /// Executable to start.
    pub executable: PathBuf,
    /// Arguments passed to the executable.
    pub args: Vec<String>,
}

impl VisualClientCommand {
    /// Start the game client as a separate process.
    pub fn spawn(&self) -> Result<(), LaunchFailure> {
        Command::new(&self.executable)
            .args(&self.args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .map(|_| ())
            .map_err(|err| {
                LaunchFailure(format!(
                    "Could not start {}: {err}",
                    self.executable.display()
                ))
            })
    }
}

/// Executable names that can host both the launcher and the game-client entry point.
const HOST_NAMES: [&str; 4] = [
    "legohans-client.exe",
    "legohans-client",
    "lc-app.exe",
    "lc-app",
];

/// Find a game-client host executable next to `dir`, preferring the no-console binary.
fn find_host_in(dir: &Path) -> Option<PathBuf> {
    HOST_NAMES
        .iter()
        .map(|name| dir.join(name))
        .find(|candidate| candidate.is_file())
}

/// Build the command that opens the visual-only game client.
///
/// The launcher must not start its test harness or an unrelated binary, so the current
/// executable is only used when it is one of the known host names.
pub fn visual_client_command() -> Result<VisualClientCommand, LaunchFailure> {
    let current = env::current_exe()
        .map_err(|err| LaunchFailure(format!("Could not read the current executable: {err}")))?;
    let dir = current
        .parent()
        .ok_or_else(|| LaunchFailure("The current executable has no parent directory.".into()))?
        .to_path_buf();
    let current_is_host = current
        .file_name()
        .and_then(OsStr::to_str)
        .is_some_and(|name| HOST_NAMES.contains(&name));
    let executable = find_host_in(&dir)
        .or_else(|| current_is_host.then_some(current))
        .ok_or_else(|| {
            LaunchFailure(format!(
                "No game client executable was found in {}.",
                dir.display()
            ))
        })?;
    Ok(VisualClientCommand {
        executable,
        args: vec!["--ingame-ui".into(), "loading".into()],
    })
}

/// Command builder used by [`VisualClientBackend`].
type CommandFn = dyn Fn() -> Result<VisualClientCommand, LaunchFailure>;

/// Spawn function used by [`VisualClientBackend`].
type SpawnFn = dyn Fn(&VisualClientCommand) -> Result<(), LaunchFailure>;

/// The game backend that opens the implemented visual-only game-client window.
///
/// Command building and spawning are injectable so tests never open a real window.
pub struct VisualClientBackend {
    command: Box<CommandFn>,
    spawn: Box<SpawnFn>,
}

impl Default for VisualClientBackend {
    fn default() -> Self {
        Self {
            command: Box::new(visual_client_command),
            spawn: Box::new(VisualClientCommand::spawn),
        }
    }
}

impl VisualClientBackend {
    /// Create a backend with an injected command builder and spawn function.
    #[cfg(test)]
    pub fn with_command_and_spawn<C, F>(command: C, spawn: F) -> Self
    where
        C: Fn() -> Result<VisualClientCommand, LaunchFailure> + 'static,
        F: Fn(&VisualClientCommand) -> Result<(), LaunchFailure> + 'static,
    {
        Self {
            command: Box::new(command),
            spawn: Box::new(spawn),
        }
    }
}

impl GameBackend for VisualClientBackend {
    fn status(&self) -> BackendStatus {
        BackendStatus::Ready
    }

    fn launch(&mut self, _request: &LaunchRequest) -> Result<(), LaunchFailure> {
        let command = (self.command)()?;
        (self.spawn)(&command)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    fn request() -> LaunchRequest {
        LaunchRequest {
            profile: Profile::new("x"),
            account: AccountKind::Offline {
                username: "Alex".into(),
            },
            server: None,
            game_dir: PathBuf::from("."),
        }
    }

    #[test]
    fn visual_backend_is_ready() {
        let backend = VisualClientBackend::default();
        assert_eq!(backend.status(), BackendStatus::Ready);
    }

    fn stub_command() -> Result<VisualClientCommand, LaunchFailure> {
        Ok(VisualClientCommand {
            executable: PathBuf::from("client.exe"),
            args: vec!["--ingame-ui".into(), "loading".into()],
        })
    }

    #[test]
    fn launch_starts_the_loading_screen() {
        let captured = Rc::new(RefCell::new(Vec::new()));
        let sink = captured.clone();
        let mut backend =
            VisualClientBackend::with_command_and_spawn(stub_command, move |command| {
                sink.borrow_mut().push(command.clone());
                Ok(())
            });
        backend.launch(&request()).unwrap();
        let commands = captured.borrow();
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].args, ["--ingame-ui", "loading"]);
    }

    #[test]
    fn prefers_no_console_host_when_present() {
        let dir = env::temp_dir().join(format!("lc-launch-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        for name in ["lc-app.exe", "legohans-client.exe"] {
            std::fs::write(dir.join(name), b"stub").unwrap();
        }
        let found = find_host_in(&dir).unwrap();
        assert_eq!(found.file_name().unwrap(), "legohans-client.exe");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn launch_reports_spawn_failure() {
        let mut backend = VisualClientBackend::with_command_and_spawn(stub_command, |_| {
            Err(LaunchFailure("could not start".into()))
        });
        assert_eq!(
            backend.launch(&request()),
            Err(LaunchFailure("could not start".into()))
        );
    }
}
