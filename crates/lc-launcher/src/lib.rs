//! Desktop launcher for Legohan's Client.
//!
//! The launcher manages installations, accounts, servers, resource packs and screenshots and
//! starts the game through the [`backend::GameBackend`] boundary. The Rust game client does not
//! exist yet, so launching is reported as unavailable rather than faked.

mod app;
mod backend;
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

/// A quad (four corners) and its RGB color.
type Face = ([(f32, f32); 4], [u8; 3]);

/// Build the 64x64 window icon: an isometric amethyst block.
fn app_icon() -> IconData {
    const N: usize = 64;
    let mut rgba = vec![0u8; N * N * 4];
    let c = N as f32 / 2.0;
    let r = c - 3.0;
    let p = |x: f32, y: f32| (c + x * r, c + y * r);
    // Faces as quads: (top, left, right) with colors.
    let faces: [Face; 3] = [
        (
            [p(0.0, -1.0), p(0.866, -0.5), p(0.0, 0.0), p(-0.866, -0.5)],
            [186, 130, 240],
        ),
        (
            [p(-0.866, -0.5), p(0.0, 0.0), p(0.0, 1.0), p(-0.866, 0.5)],
            [138, 90, 196],
        ),
        (
            [p(0.0, 0.0), p(0.866, -0.5), p(0.866, 0.5), p(0.0, 1.0)],
            [108, 68, 164],
        ),
    ];
    fn inside(poly: &[(f32, f32); 4], x: f32, y: f32) -> bool {
        let mut sign = 0.0f32;
        for i in 0..4 {
            let (ax, ay) = poly[i];
            let (bx, by) = poly[(i + 1) % 4];
            let cross = (bx - ax) * (y - ay) - (by - ay) * (x - ax);
            if cross != 0.0 {
                if sign == 0.0 {
                    sign = cross.signum();
                } else if sign != cross.signum() {
                    return false;
                }
            }
        }
        true
    }
    for y in 0..N {
        for x in 0..N {
            for (poly, color) in &faces {
                if inside(poly, x as f32 + 0.5, y as f32 + 0.5) {
                    let i = (y * N + x) * 4;
                    rgba[i..i + 4].copy_from_slice(&[color[0], color[1], color[2], 255]);
                }
            }
        }
    }
    IconData {
        rgba,
        width: N as u32,
        height: N as u32,
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
