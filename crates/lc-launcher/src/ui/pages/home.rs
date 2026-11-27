//! Home: artwork hero with the selected installation and the Play button.

use std::path::PathBuf;

use egui::{Align, Layout, Rect, RichText, UiBuilder, Vec2, vec2};

use super::{Ctx, backend_chip, last_played_text, launch_and_report};
use crate::model::settings::HomeBackground;
use crate::paths::default_minecraft_dir;
use crate::services::images::ImageSource;
use crate::services::panorama::find_client_jar;
use crate::ui::Page;
use crate::ui::artwork::{paint_landscape, paint_panorama};
use crate::ui::icons::paint_block_icon;
use crate::ui::widgets::{glass_panel, gradient_h, gradient_v, primary_button, secondary_button};
use crate::util::now_secs;

const PANORAMA_KEY: &str = "home-panorama";

/// Home page state.
#[derive(Default)]
pub struct State {
    resolved_for: Option<(HomeBackground, String)>,
    jar: Option<PathBuf>,
}

fn locate_jar(c: &Ctx<'_>) -> Option<PathBuf> {
    let settings = &c.state.data.settings;
    let dir = if settings.minecraft_dir.trim().is_empty() {
        default_minecraft_dir(std::env::consts::OS, |k| {
            std::env::var(k).ok().filter(|v| !v.is_empty())
        })?
    } else {
        PathBuf::from(settings.minecraft_dir.trim())
    };
    find_client_jar(&dir, lc_protocol::TARGET_GAME_VERSION)
}

/// Draw the Home page.
pub fn show(ui: &mut egui::Ui, c: &mut Ctx<'_>) {
    let rect = ui.max_rect();

    // Decide where the artwork comes from, once per relevant setting change.
    let key = (
        c.state.data.settings.home_background,
        c.state.data.settings.minecraft_dir.clone(),
    );
    if c.ui.home.resolved_for.as_ref() != Some(&key) {
        c.ui.home.jar = match key.0 {
            HomeBackground::Auto => locate_jar(c),
            HomeBackground::Generated => None,
        };
        c.textures.evict(PANORAMA_KEY);
        c.ui.home.resolved_for = Some(key);
    }
    if let Some(jar) = c.ui.home.jar.clone() {
        c.textures
            .request(PANORAMA_KEY, ImageSource::Panorama(jar), 512);
    }

    let time = if c.theme.animations { c.now } else { 0.0 };
    let painter = ui.painter_at(rect);
    match c.textures.get(PANORAMA_KEY) {
        Some(tex) => paint_panorama(&painter, rect, tex, time),
        None => paint_landscape(&painter, rect, time, c.theme.accent),
    }
    if c.theme.animations {
        c.egui
            .request_repaint_after(std::time::Duration::from_millis(33));
    }
    // Darken for legibility.
    let dark = egui::Color32::from_rgba_unmultiplied(6, 4, 12, 225);
    let clear = egui::Color32::from_rgba_unmultiplied(6, 4, 12, 20);
    gradient_h(&painter, rect, dark, clear);
    gradient_v(
        &painter,
        Rect::from_min_max(
            egui::pos2(rect.left(), rect.top() + rect.height() * 0.55),
            rect.max,
        ),
        egui::Color32::TRANSPARENT,
        egui::Color32::from_rgba_unmultiplied(8, 6, 14, 230),
    );

    let content = rect.shrink2(vec2(56.0, 44.0));
    let mut inner = ui.new_child(
        UiBuilder::new()
            .max_rect(content)
            .layout(Layout::top_down(Align::Min)),
    );
    let ui = &mut inner;
    let hero_height = 392.0;
    ui.add_space((ui.available_height() - hero_height).max(0.0));

    ui.label(
        RichText::new("MINECRAFT JAVA EDITION 1.21.11")
            .size(13.0)
            .strong()
            .color(c.theme.accent_hi),
    );
    ui.label(
        RichText::new("Legohan's Client")
            .size(58.0)
            .strong()
            .color(egui::Color32::WHITE),
    );
    ui.label(
        RichText::new("A complete Minecraft client, written in Rust.")
            .size(17.0)
            .color(c.theme.text_dim),
    );
    ui.add_space(18.0);

    // Selected installation.
    let profile = c.state.data.selected_profile().cloned();
    let now = now_secs();
    glass_panel(ui, c.theme, |ui| {
        ui.set_width(380.0);
        ui.horizontal(|ui| {
            match &profile {
                Some(p) => {
                    let (icon_rect, _) =
                        ui.allocate_exact_size(Vec2::splat(58.0), egui::Sense::hover());
                    paint_block_icon(ui.painter(), icon_rect, p.icon, c.theme.accent_alpha(60));
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(&p.name)
                                .size(19.0)
                                .strong()
                                .color(c.theme.text),
                        );
                        ui.label(
                            RichText::new(format!(
                                "Version {}  ·  {}",
                                p.version,
                                last_played_text(p.last_played, now)
                            ))
                            .size(13.5)
                            .color(c.theme.text_dim),
                        );
                    });
                }
                None => {
                    ui.label(RichText::new("No installation yet").color(c.theme.text_dim));
                }
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if secondary_button(ui, c.theme, "Change").clicked() {
                    c.ui.next_page = Some(Page::Installations);
                }
            });
        });
    });
    ui.add_space(20.0);

    // Play.
    let ready = matches!(
        c.state.backend_status(),
        crate::backend::BackendStatus::Ready
    );
    ui.horizontal(|ui| {
        let resp = primary_button(ui, c.theme, "PLAY", vec2(300.0, 70.0), ready);
        if resp.clicked() {
            launch_and_report(c, None);
        }
        ui.vertical(|ui| {
            ui.add_space(6.0);
            backend_chip(ui, c);
            let who = match c.state.data.selected_account() {
                Some(a) => format!(
                    "Playing as {} ({})",
                    a.username(),
                    a.status_label().to_lowercase()
                ),
                None => "No account yet. Add one on the Accounts page.".to_string(),
            };
            ui.label(RichText::new(who).size(13.5).color(c.theme.text_dim));
        });
    });
}
