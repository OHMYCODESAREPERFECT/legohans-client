//! Legohan's Client: command-line entry point.
//!
//! With no arguments this opens the launcher window. `--version` and `--help` print text and
//! exit, which makes this binary usable from scripts and CI. `--ingame-ui [screen]` opens the
//! separate visual-only in-game menu preview. The `legohans-client` binary opens the launcher
//! without a console window on Windows.

use std::process::ExitCode;

use lc_ui::ingame::state::Screen;

const NAME: &str = "Legohan's Client";
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// What the command line asks for.
#[derive(Debug, PartialEq, Eq)]
enum Action {
    /// Print this text and exit successfully.
    Print(String),
    /// Open the launcher window.
    RunLauncher,
    /// Open the visual-only in-game menu preview.
    RunIngameUi(Screen),
}

fn screen_names() -> String {
    Screen::ALL
        .iter()
        .map(|screen| screen.arg_name())
        .collect::<Vec<_>>()
        .join(", ")
}

/// Decide what to do for the given command-line arguments (without the program name).
fn parse_args(args: &[String]) -> Result<Action, String> {
    match args.first().map(String::as_str) {
        None => Ok(Action::RunLauncher),
        Some("--version" | "-V") => Ok(Action::Print(format!(
            "{NAME} {VERSION} (targets Minecraft Java Edition {})",
            lc_net::GAME_VERSION
        ))),
        Some("--help" | "-h") => Ok(Action::Print(format!(
            "{NAME} {VERSION}\n\nUSAGE:\n    lc-app [--version | --help | --ingame-ui [screen]]\n\nWith no arguments the launcher window opens.\n--ingame-ui opens the visual-only menu preview. Available screens: {}.\nThe game client itself is not implemented yet, so the launcher cannot start a game.",
            screen_names()
        ))),
        Some("--ingame-ui") => {
            let screen = match args.get(1).map(String::as_str) {
                None => Screen::MainMenu,
                Some(name) => Screen::from_arg(name).ok_or_else(|| {
                    format!(
                        "unknown in-game screen: {name}\nAvailable screens: {}.",
                        screen_names()
                    )
                })?,
            };
            Ok(Action::RunIngameUi(screen))
        }
        Some(arg) if arg.starts_with("--ingame-ui=") => {
            let name = &arg["--ingame-ui=".len()..];
            let screen = Screen::from_arg(name).ok_or_else(|| {
                format!(
                    "unknown in-game screen: {name}\nAvailable screens: {}.",
                    screen_names()
                )
            })?;
            Ok(Action::RunIngameUi(screen))
        }
        Some(other) => Err(format!("unknown argument: {other}\nTry `lc-app --help`.")),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match parse_args(&args) {
        Ok(Action::Print(text)) => {
            println!("{text}");
            ExitCode::SUCCESS
        }
        Ok(Action::RunLauncher) => match lc_launcher::run() {
            Ok(()) => ExitCode::SUCCESS,
            Err(message) => {
                eprintln!("The launcher could not start: {message}");
                ExitCode::FAILURE
            }
        },
        Ok(Action::RunIngameUi(screen)) => match lc_ui::ingame::run(screen) {
            Ok(()) => ExitCode::SUCCESS,
            Err(message) => {
                eprintln!("The in-game UI preview could not start: {message}");
                ExitCode::FAILURE
            }
        },
        Err(message) => {
            eprintln!("{message}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn no_arguments_opens_the_launcher() {
        assert_eq!(parse_args(&[]), Ok(Action::RunLauncher));
    }

    #[test]
    fn version_prints_name_and_target() {
        match parse_args(&args(&["--version"])).unwrap() {
            Action::Print(text) => {
                assert!(text.contains("Legohan's Client"));
                assert!(text.contains("1.21.11"));
            }
            other => panic!("unexpected {other:?}"),
        }
        assert_eq!(
            parse_args(&args(&["-V"])),
            parse_args(&args(&["--version"]))
        );
    }

    #[test]
    fn help_mentions_usage_and_the_launcher() {
        match parse_args(&args(&["--help"])).unwrap() {
            Action::Print(text) => {
                assert!(text.contains("USAGE"));
                assert!(text.contains("launcher"));
                assert!(text.contains("--ingame-ui"));
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn ingame_ui_defaults_to_the_main_menu() {
        assert_eq!(
            parse_args(&args(&["--ingame-ui"])),
            Ok(Action::RunIngameUi(Screen::MainMenu))
        );
    }

    #[test]
    fn ingame_ui_accepts_named_screens() {
        assert_eq!(
            parse_args(&args(&["--ingame-ui", "options"])),
            Ok(Action::RunIngameUi(Screen::Options))
        );
        assert_eq!(
            parse_args(&args(&["--ingame-ui=pause"])),
            Ok(Action::RunIngameUi(Screen::Pause))
        );
    }

    #[test]
    fn unknown_screen_is_an_error() {
        let error = parse_args(&args(&["--ingame-ui", "nope"])).unwrap_err();
        assert!(error.contains("unknown in-game screen"));
    }

    #[test]
    fn unknown_argument_is_an_error() {
        assert!(parse_args(&args(&["--bogus"])).is_err());
    }
}
