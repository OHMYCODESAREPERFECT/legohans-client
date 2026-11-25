//! Legohan's Client: application entry point.
//!
//! Task 001 only provides a minimal command-line shell: it prints version
//! information and exits. There is no window, networking or gameplay yet.

use std::process::ExitCode;

const NAME: &str = "Legohan's Client";
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Build the text printed for the given command-line arguments (without the program name).
fn render_output(args: &[String]) -> Result<String, String> {
    match args.first().map(String::as_str) {
        None | Some("--version" | "-V") => Ok(format!(
            "{NAME} {VERSION} (targets Minecraft Java Edition {})",
            lc_net::GAME_VERSION
        )),
        Some("--help" | "-h") => Ok(format!(
            "{NAME} {VERSION}\n\nUSAGE:\n    lc-app [--version | --help]\n\nNo gameplay is implemented yet."
        )),
        Some(other) => Err(format!("unknown argument: {other}\nTry `lc-app --help`.")),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match render_output(&args) {
        Ok(text) => {
            println!("{text}");
            ExitCode::SUCCESS
        }
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
    fn default_prints_version_and_target() {
        let out = render_output(&[]).unwrap();
        assert!(out.contains("Legohan's Client"));
        assert!(out.contains("1.21.11"));
    }

    #[test]
    fn version_flag_matches_default() {
        assert_eq!(render_output(&args(&["--version"])), render_output(&[]));
    }

    #[test]
    fn help_mentions_usage() {
        assert!(render_output(&args(&["--help"])).unwrap().contains("USAGE"));
    }

    #[test]
    fn unknown_argument_is_an_error() {
        assert!(render_output(&args(&["--bogus"])).is_err());
    }
}
