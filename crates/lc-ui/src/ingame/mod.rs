//! Visual-only Minecraft Java Edition menu shell.
//!
//! This module is intentionally separate from the launcher. It embeds the supplied official
//! artwork, renders it with nearest-neighbor sampling, and never loads a world, connects to a
//! server, authenticates an account, or changes saved settings.

pub mod assets;
pub mod font;
pub mod panorama;
pub mod screens;
pub mod state;
pub mod widgets;

use egui::{CornerRadius, ViewportBuilder};
use state::Screen;

use self::{
    panorama::Panorama,
    state::MenuState,
    widgets::{Visuals, gui_scale},
};

/// A native window containing the visual-only in-game menu.
pub struct IngameApp {
    state: MenuState,
    visuals: Visuals,
    panorama: Panorama,
    last_time: Option<f64>,
}

impl IngameApp {
    /// Build the app for one starting screen.
    pub fn new(ctx: &egui::Context, screen: Screen) -> Self {
        let visuals = Visuals::new(ctx);
        let panorama = Panorama::new(ctx);
        let mut state = MenuState::new(screen);
        if let Ok(yaw) = std::env::var("LC_INGAME_YAW") {
            state.yaw_override = yaw.parse().ok();
        }
        if let Ok(pitch) = std::env::var("LC_INGAME_PITCH") {
            state.pitch_override = pitch.parse().ok();
        }
        Self {
            state,
            visuals,
            panorama,
            last_time: None,
        }
    }
}

impl eframe::App for IngameApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let now = ctx.input(|input| input.time);
        let dt = self.last_time.map_or(0.0, |last| (now - last) as f32);
        self.last_time = Some(now);
        self.state.tick(dt);
        if ctx.input(|input| input.key_pressed(egui::Key::Escape)) {
            self.state.back();
        }
        ctx.request_repaint();

        let rect = ctx.content_rect();
        self.visuals.scale = gui_scale(rect.size());
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                ui.painter()
                    .rect_filled(ui.max_rect(), CornerRadius::ZERO, egui::Color32::BLACK);
                let visuals = self.visuals.clone();
                screens::show(
                    ui,
                    ui.max_rect(),
                    &mut self.state,
                    &visuals,
                    &mut self.panorama,
                );
            });
    }
}

fn env_f32(name: &str, fallback: f32) -> f32 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .filter(|value: &f32| value.is_finite() && *value > 0.0)
        .unwrap_or(fallback)
}

/// Open the visual-only in-game menu window.
///
/// `screen` chooses the screen visible on launch. Environment variables `LC_INGAME_YAW`,
/// `LC_INGAME_PITCH`, `LC_INGAME_WIDTH` and `LC_INGAME_HEIGHT` allow deterministic screenshots.
pub fn run(screen: Screen) -> Result<(), String> {
    let width = env_f32("LC_INGAME_WIDTH", 1280.0);
    let height = env_f32("LC_INGAME_HEIGHT", 720.0);
    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_title("Legohan's Client - In-Game UI Preview")
            .with_inner_size([width, height])
            .with_min_inner_size([480.0, 320.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Legohan's Client - In-Game UI Preview",
        options,
        Box::new(move |cc| Ok(Box::new(IngameApp::new(&cc.egui_ctx, screen)))),
    )
    .map_err(|err| err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starting_screens_are_covered() {
        assert_eq!(Screen::ALL.len(), 12);
        assert!(Screen::ALL.contains(&Screen::MainMenu));
        assert!(Screen::ALL.contains(&Screen::Pause));
    }

    fn frame(ctx: &egui::Context, app: &mut IngameApp, time: f64, size: (f32, f32)) {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(size.0, size.1),
            )),
            time: Some(time),
            ..Default::default()
        };
        let mut frame = eframe::Frame::_new_kittest();
        let _ = ctx.run(input, |ctx| eframe::App::update(app, ctx, &mut frame));
    }

    #[test]
    fn every_screen_renders_at_several_sizes() {
        let ctx = egui::Context::default();
        let mut app = IngameApp::new(&ctx, Screen::MainMenu);
        let mut time = 0.0;
        for size in [
            (1280.0, 720.0),
            (800.0, 500.0),
            (1920.0, 1080.0),
            (480.0, 320.0),
        ] {
            for screen in Screen::ALL {
                app.state.screen = screen;
                app.state.transition = 0.0;
                for _ in 0..3 {
                    time += 0.1;
                    frame(&ctx, &mut app, time, size);
                }
                assert_eq!(app.state.screen, screen);
            }
        }
    }
}
