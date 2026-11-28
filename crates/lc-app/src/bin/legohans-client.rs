//! Legohan's Client launcher without a console window on Windows.

#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use std::process::ExitCode;

use lc_ui::ingame::state::Screen;

/// Return the screen requested by `--ingame-ui`, if present.
fn ingame_screen(args: &[String]) -> Result<Option<Screen>, String> {
    let name = match args.first().map(String::as_str) {
        None => return Ok(None),
        Some("--ingame-ui") => args.get(1).map(String::as_str).unwrap_or("main-menu"),
        Some(arg) if arg.starts_with("--ingame-ui=") => &arg["--ingame-ui=".len()..],
        Some(_) => return Ok(None),
    };
    Screen::from_arg(name)
        .map(Some)
        .ok_or_else(|| format!("unknown in-game screen: {name}"))
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match ingame_screen(&args) {
        Ok(Some(screen)) => match lc_ui::ingame::run(screen) {
            Ok(()) => ExitCode::SUCCESS,
            Err(message) => {
                eprintln!("The game client could not start: {message}");
                ExitCode::FAILURE
            }
        },
        Ok(None) => match lc_launcher::run() {
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
    fn launcher_is_the_default() {
        assert_eq!(ingame_screen(&[]), Ok(None));
    }

    #[test]
    fn ingame_ui_selects_a_screen() {
        assert_eq!(
            ingame_screen(&args(&["--ingame-ui", "singleplayer"])),
            Ok(Some(Screen::Singleplayer))
        );
        assert_eq!(
            ingame_screen(&args(&["--ingame-ui=video"])),
            Ok(Some(Screen::Video))
        );
    }

    #[test]
    fn invalid_screen_is_rejected() {
        assert!(ingame_screen(&args(&["--ingame-ui", "bad"])).is_err());
    }
}
