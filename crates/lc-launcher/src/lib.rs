//! Desktop launcher for Legohan's Client.
//!
//! The launcher manages installations, accounts, servers, resource packs and screenshots and
//! starts the game through the [`backend::GameBackend`] boundary. The Rust game client does not
//! exist yet, so launching is reported as unavailable rather than faked.

mod app;
mod backend;
mod icon;
mod model;
mod paths;
mod services;
mod startup;
mod state;
mod store;
mod ui;
mod updates;
mod util;

#[cfg(test)]
mod test_support;

use egui::{IconData, ViewportBuilder};

use app::LauncherApp;
use backend::NativeClientBackend;
use model::LauncherData;
use paths::LauncherDirs;
use state::LauncherState;

/// Build the 64x64 window icon: a black isometric cube with an "LC" mark.
fn app_icon() -> IconData {
    IconData {
        rgba: icon::rgba(),
        width: icon::SIZE as u32,
        height: icon::SIZE as u32,
    }
}

/// Open the launcher window and run until it is closed.
pub fn run() -> Result<(), String> {
    let dirs = LauncherDirs::detect();
    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_title("Legohan's Client")
            .with_inner_size([1280.0, 780.0])
            .with_min_inner_size([760.0, 520.0])
            .with_icon(app_icon())
            .with_drag_and_drop(true),
        ..Default::default()
    };
    eframe::run_native(
        "Legohan's Client",
        options,
        Box::new(move |cc| {
            let data = LauncherData::load_settings(dirs);
            let state = LauncherState::new(data, Box::new(NativeClientBackend));
            Ok(Box::new(LauncherApp::new(&cc.egui_ctx, state)))
        }),
    )
    .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_has_expected_size_and_opaque_pixels() {
        let icon = app_icon();
        assert_eq!(icon.rgba.len(), (icon.width * icon.height * 4) as usize);
        let opaque = icon.rgba.chunks_exact(4).filter(|p| p[3] == 255).count();
        assert!(opaque > 1000, "the block should cover a large area");
        assert_eq!(icon.rgba[3], 0, "corners stay transparent");
    }
}
