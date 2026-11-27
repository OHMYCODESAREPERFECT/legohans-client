//! Legohan's Client launcher without a console window on Windows release builds.

#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use std::process::ExitCode;

fn main() -> ExitCode {
    match lc_launcher::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("The launcher could not start: {message}");
            ExitCode::FAILURE
        }
    }
}
