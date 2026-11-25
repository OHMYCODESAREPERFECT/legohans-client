//! Developer tooling entry point. Run with `cargo xtask`.
//!
//! No tasks exist yet. Data generation from the user's own Minecraft install
//! will be added here later; Mojang assets are never committed to this repo.

use std::process::ExitCode;

fn main() -> ExitCode {
    let task = std::env::args().nth(1);
    match task.as_deref() {
        None | Some("--help" | "-h") => {
            println!("xtask: no tasks are available yet.");
            ExitCode::SUCCESS
        }
        Some(other) => {
            eprintln!("xtask: unknown task `{other}`");
            ExitCode::from(2)
        }
    }
}
