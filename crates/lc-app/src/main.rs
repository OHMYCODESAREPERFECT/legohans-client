//! Legohan's Client: command-line entry point.
//!
//! With no arguments this opens the launcher window. `--version` and `--help` print text and
//! exit, which makes this binary usable from scripts and CI. The `legohans-client` binary
//! opens the launcher without a console window on Windows.

use std::process::ExitCode;

const NAME: &str = "Legohan's Client";
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// What the command line asks for.
#[derive(Debug, PartialEq, Eq)]
enum Action {
    /// Print this text and exit successfully.
    Print(String),
    /// Open the launcher window.
    RunLauncher,
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
            "{NAME} {VERSION}\n\nUSAGE:\n    lc-app [--version | --help]\n\nWith no arguments the launcher window opens.\nThe game client itself is not implemented yet, so the launcher cannot start a game."
        ))),
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
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn unknown_argument_is_an_error() {
        assert!(parse_args(&args(&["--bogus"])).is_err());
    }
}
