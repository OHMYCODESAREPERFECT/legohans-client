//! Settings: categorized, searchable launcher settings.

use std::path::PathBuf;

use egui::{Key, Margin, RichText, Sense, Shape, Stroke, vec2};

use super::{Ctx, caption, confirm_modal};
use crate::model::color::{Hsv, format_hex_rgb, parse_hex_rgb};
use crate::model::settings::{
    Accent, AfterLaunch, Category, HomeBackground, SettingId, SettingMeta, Settings,
    parse_ui_scale, search,
};
use crate::services::images::ImageSource;
use crate::services::panorama;
use crate::services::panorama::find_client_jar;
use crate::ui::theme::Theme;
use crate::ui::toasts::ToastKind;
use crate::ui::widgets::{
    glass_frame, page_header, secondary_button, section_title, toggle, wide_button,
};
use crate::util::{open_folder, pick_png, pick_pngs};

const SETTINGS_PANORAMA_KEY: &str = "settings-panorama";

/// Settings page state.
pub struct State {
    query: String,
    confirm_reset: bool,
    scale_text: String,
    scale_error: Option<String>,
    scale_focused: bool,
    wheel_hsv: Hsv,
    custom_hex: String,
    hex_error: Option<String>,
    hex_focused: bool,
    panorama_sources: [Option<PathBuf>; 4],
    panorama_rev: Option<u64>,
}

impl Default for State {
    fn default() -> Self {
        let accent = Accent::default().color();
        Self {
            query: String::new(),
            confirm_reset: false,
            scale_text: "100%".to_string(),
            scale_error: None,
            scale_focused: false,
            wheel_hsv: Hsv::from_rgb(accent),
            custom_hex: format_hex_rgb(accent),
            hex_error: None,
            hex_focused: false,
            panorama_sources: [None, None, None, None],
            panorama_rev: None,
        }
    }
}

/// Draw the Settings page.
pub fn show(ui: &mut egui::Ui, c: &mut Ctx<'_>) {
    sync_controls(c);
    page_header(
        ui,
        c.theme,
        "Settings",
        "Changes apply immediately and are saved automatically.",
    );
    ui.add(
        egui::TextEdit::singleline(&mut c.ui.settings.query)
            .hint_text("Search settings (for example: zoom, update, folder)")
            .desired_width(ui.available_width().min(520.0)),
    );
    ui.add_space(6.0);

    let results = search(&c.ui.settings.query);
    if results.is_empty() {
        ui.add_space(12.0);
        ui.label(
            RichText::new("No settings match your search.")
                .size(16.0)
                .color(c.theme.text_dim),
        );
        return;
    }

    egui::ScrollArea::vertical()
        .id_salt("settings_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for category in Category::ALL {
                let items: Vec<&SettingMeta> = results
                    .iter()
                    .copied()
                    .filter(|m| m.category == category)
                    .collect();
                if items.is_empty() {
                    continue;
                }
                section_title(ui, c.theme, category.title());
                for meta in items {
                    row(ui, c, meta);
                }
                ui.add_space(10.0);
            }
        });

    if c.ui.settings.confirm_reset {
        match confirm_modal(
            c.egui,
            c.theme,
            "reset_settings",
            "Reset settings?",
            "All launcher settings return to their defaults. Installations, accounts, servers and packs are kept.",
            "Reset",
        ) {
            Some(true) => {
                let keep_profile = c.state.data.settings.selected_profile.clone();
                let keep_account = c.state.data.settings.selected_account.clone();
                c.state.data.settings = Settings {
                    selected_profile: keep_profile,
                    selected_account: keep_account,
                    ..Settings::default()
                };
                c.ui.settings.confirm_reset = false;
                let st = &mut c.ui.settings;
                st.scale_focused = false;
                st.scale_error = None;
                st.hex_focused = false;
                st.hex_error = None;
                c.toasts
                    .push(ToastKind::Info, "Settings reset to defaults.");
            }
            Some(false) => c.ui.settings.confirm_reset = false,
            None => {}
        }
    }
}

fn sync_controls(c: &mut Ctx<'_>) {
    let settings = &c.state.data.settings;
    let state = &mut c.ui.settings;
    if !state.scale_focused && state.scale_error.is_none() {
        state.scale_text = format!("{:.0}%", settings.ui_scale * 100.0);
    }
    if !state.hex_focused && state.hex_error.is_none() {
        let rgb = settings.accent.color();
        state.wheel_hsv = Hsv::from_rgb(rgb);
        state.custom_hex = format_hex_rgb(rgb);
    }
    if state.panorama_rev != Some(settings.custom_panorama_rev) {
        c.textures.evict(SETTINGS_PANORAMA_KEY);
        state.panorama_rev = Some(settings.custom_panorama_rev);
    }
}

fn row(ui: &mut egui::Ui, c: &mut Ctx<'_>, meta: &SettingMeta) {
    glass_frame(c.theme)
        .inner_margin(Margin::symmetric(16, 12))
        .corner_radius(12)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.set_width((ui.available_width() * 0.5).max(180.0));
                    ui.label(
                        RichText::new(meta.title)
                            .size(15.5)
                            .strong()
                            .color(c.theme.text),
                    );
                    ui.label(
                        RichText::new(meta.description)
                            .size(13.0)
                            .color(c.theme.text_dim),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    control(ui, c, meta.id);
                });
            });
        });
    ui.add_space(4.0);
}

fn control(ui: &mut egui::Ui, c: &mut Ctx<'_>, id: SettingId) {
    let theme = c.theme.clone();
    match id {
        SettingId::Accent => accent_control(ui, c),
        SettingId::UiScale => scale_control(ui, c),
        SettingId::Animations => {
            toggle(ui, &theme, &mut c.state.data.settings.animations);
        }
        SettingId::CompactSidebar => {
            toggle(ui, &theme, &mut c.state.data.settings.compact_sidebar);
        }
        SettingId::ShowSplash => {
            toggle(ui, &theme, &mut c.state.data.settings.show_splash);
        }
        SettingId::CheckUpdates => {
            toggle(ui, &theme, &mut c.state.data.settings.check_updates);
        }
        SettingId::HomeBackground => {
            let s = &mut c.state.data.settings;
            egui::ComboBox::from_id_salt("home_bg")
                .selected_text(s.home_background.name())
                .show_ui(ui, |ui| {
                    for b in HomeBackground::ALL {
                        ui.selectable_value(&mut s.home_background, b, b.name());
                    }
                });
            panorama_control(ui, c);
        }
        SettingId::AfterLaunch => {
            let s = &mut c.state.data.settings;
            egui::ComboBox::from_id_salt("after_launch")
                .selected_text(s.after_launch.name())
                .show_ui(ui, |ui| {
                    for a in AfterLaunch::ALL {
                        ui.selectable_value(&mut s.after_launch, a, a.name());
                    }
                });
        }
        SettingId::MinecraftDir => {
            let s = &mut c.state.data.settings;
            ui.vertical(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut s.minecraft_dir)
                        .hint_text("Leave empty to detect automatically")
                        .desired_width(320.0),
                );
                let dir = if s.minecraft_dir.trim().is_empty() {
                    crate::paths::default_minecraft_dir(std::env::consts::OS, |k| {
                        std::env::var(k).ok().filter(|v| !v.is_empty())
                    })
                } else {
                    Some(std::path::PathBuf::from(s.minecraft_dir.trim()))
                };
                let found = dir
                    .as_deref()
                    .and_then(|d| find_client_jar(d, lc_protocol::TARGET_GAME_VERSION))
                    .is_some();
                caption(
                    ui,
                    &theme,
                    if found {
                        "Found the 1.21.11 client jar. Its panorama can be used on Home."
                    } else {
                        "No 1.21.11 client jar found there, so the generated landscape is used."
                    },
                );
            });
        }
        SettingId::DataFolder => {
            folder_control(ui, c, &theme, c.state.data.dirs.root().to_path_buf())
        }
        SettingId::ScreenshotsFolder => {
            folder_control(ui, c, &theme, c.state.data.dirs.screenshots_dir())
        }
        SettingId::Reset => {
            if wide_button(ui, &theme, "Reset settings…", 0.0).clicked() {
                c.ui.settings.confirm_reset = true;
            }
        }
        SettingId::About => {
            ui.vertical(|ui| {
                ui.label(
                    RichText::new(format!("Version {}", env!("CARGO_PKG_VERSION")))
                        .color(theme.text),
                );
                caption(
                    ui,
                    &theme,
                    &format!(
                        "Targets Minecraft Java Edition {} (protocol {})",
                        lc_protocol::TARGET_GAME_VERSION,
                        lc_protocol::PROTOCOL_VERSION
                    ),
                );
                caption(ui, &theme, "License: not chosen yet");
            });
        }
    }
}

fn accent_control(ui: &mut egui::Ui, c: &mut Ctx<'_>) {
    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            let current = c.state.data.settings.accent;
            for accent in Accent::ALL.iter().rev() {
                let color = Theme::new(*accent, true).accent;
                let (rect, resp) = ui.allocate_exact_size(vec2(28.0, 28.0), Sense::click());
                let on = current == *accent;
                if on {
                    crate::ui::widgets::glow(
                        ui.painter(),
                        rect.shrink(8.0),
                        6,
                        color.gamma_multiply(0.5),
                        16,
                    );
                }
                ui.painter().circle_filled(rect.center(), 11.0, color);
                if on || resp.hovered() {
                    ui.painter().circle_stroke(
                        rect.center(),
                        13.5,
                        Stroke::new(
                            2.0,
                            if on {
                                egui::Color32::WHITE
                            } else {
                                c.theme.border
                            },
                        ),
                    );
                }
                if resp.on_hover_text(accent.name()).clicked() {
                    c.state.data.settings.accent = *accent;
                }
            }
        });
        ui.add_space(8.0);
        wheel_picker(ui, c);
    });
}

fn wheel_picker(ui: &mut egui::Ui, c: &mut Ctx<'_>) {
    let theme = c.theme.clone();
    let mut hsv = c.ui.settings.wheel_hsv;
    let size = 116.0;
    let (rect, resp) = ui.allocate_exact_size(vec2(size, size), Sense::click_and_drag());
    let center = rect.center();
    let outer = size * 0.5 - 4.0;
    let inner = outer * 0.72;
    let painter = ui.painter();

    // Hue ring: triangular slices from the centre, with the inner disc drawn over the middle.
    for i in 0..180 {
        let a0 = i as f32 * std::f32::consts::TAU / 180.0;
        let a1 = (i + 1) as f32 * std::f32::consts::TAU / 180.0;
        let color = Hsv {
            h: a0.to_degrees(),
            s: 1.0,
            v: 1.0,
        }
        .to_rgb();
        painter.add(Shape::convex_polygon(
            vec![
                center,
                center + egui::vec2(a0.cos(), a0.sin()) * outer,
                center + egui::vec2(a1.cos(), a1.sin()) * outer,
            ],
            egui::Color32::from_rgb(color[0], color[1], color[2]),
            Stroke::NONE,
        ));
    }
    // Saturation falls towards the centre; brightness is controlled by the slider below.
    for step in 0..=16 {
        let t = step as f32 / 16.0;
        let rgb = Hsv {
            h: hsv.h,
            s: 1.0 - t,
            v: hsv.v,
        }
        .to_rgb();
        painter.circle_filled(
            center,
            inner * (1.0 - t),
            egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2]),
        );
    }
    let marker =
        center + egui::vec2(hsv.h.to_radians().cos(), hsv.h.to_radians().sin()) * inner * hsv.s;
    painter.circle_stroke(center, inner, Stroke::new(1.0, theme.border));
    painter.circle_filled(marker, 5.0, egui::Color32::WHITE);
    painter.circle_stroke(marker, 7.0, Stroke::new(1.5, theme.accent));

    if (resp.clicked() || resp.dragged())
        && let Some(pos) = resp.interact_pointer_pos()
    {
        let delta = pos - center;
        hsv.h = delta.y.atan2(delta.x).to_degrees().rem_euclid(360.0);
        hsv.s = (delta.length() / inner).clamp(0.0, 1.0);
        apply_hsv(c, hsv);
    }
    resp.on_hover_cursor(egui::CursorIcon::Crosshair);

    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.set_min_width(210.0);
        ui.label(RichText::new("Brightness").size(12.5).color(theme.text_dim));
        let mut value = hsv.v;
        if ui
            .add(egui::Slider::new(&mut value, 0.0..=1.0).show_value(false))
            .changed()
        {
            hsv.v = value;
            apply_hsv(c, hsv);
        }
        let rgb = c.state.data.settings.accent.color();
        let swatch = ui.allocate_exact_size(vec2(16.0, 16.0), Sense::hover()).0;
        ui.painter().rect_filled(
            swatch,
            egui::CornerRadius::same(4),
            egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2]),
        );
    });

    ui.horizontal(|ui| {
        ui.label(RichText::new("Hex").size(12.5).color(theme.text_dim));
        let resp = ui.add(
            egui::TextEdit::singleline(&mut c.ui.settings.custom_hex)
                .desired_width(86.0)
                .hint_text("#8B5CF6"),
        );
        if resp.gained_focus() {
            c.ui.settings.hex_focused = true;
        }
        let mut commit = resp.lost_focus();
        if commit {
            c.ui.settings.hex_focused = false;
        }
        if ui.input(|i| i.key_pressed(Key::Enter)) && c.ui.settings.hex_focused {
            resp.surrender_focus();
            commit = true;
        }
        if commit {
            match parse_hex_rgb(&c.ui.settings.custom_hex) {
                Ok(rgb) => {
                    c.state.data.settings.accent = Accent::custom(rgb);
                    c.ui.settings.wheel_hsv = Hsv::from_rgb(rgb);
                    c.ui.settings.custom_hex = format_hex_rgb(rgb);
                    c.ui.settings.hex_error = None;
                }
                Err(e) => c.ui.settings.hex_error = Some(e),
            }
        }
    });
    if let Some(error) = &c.ui.settings.hex_error {
        ui.label(RichText::new(error).size(12.0).color(theme.danger));
    }
}

fn apply_hsv(c: &mut Ctx<'_>, hsv: Hsv) {
    let rgb = hsv.to_rgb();
    c.state.data.settings.accent = Accent::custom(rgb);
    c.ui.settings.wheel_hsv = hsv;
    c.ui.settings.custom_hex = format_hex_rgb(rgb);
    c.ui.settings.hex_error = None;
}

fn scale_control(ui: &mut egui::Ui, c: &mut Ctx<'_>) {
    let theme = c.theme.clone();
    let resp = ui.add(
        egui::TextEdit::singleline(&mut c.ui.settings.scale_text)
            .desired_width(80.0)
            .hint_text("100%"),
    );
    if resp.gained_focus() {
        c.ui.settings.scale_focused = true;
    }
    let mut commit = resp.lost_focus();
    if commit {
        c.ui.settings.scale_focused = false;
    }
    if ui.input(|i| i.key_pressed(Key::Enter)) && c.ui.settings.scale_focused {
        resp.surrender_focus();
        commit = true;
    }
    if commit {
        match parse_ui_scale(&c.ui.settings.scale_text) {
            Ok(scale) => {
                c.state.data.settings.ui_scale = scale;
                c.ui.settings.scale_text = format!("{:.0}%", scale * 100.0);
                c.ui.settings.scale_error = None;
            }
            Err(e) => c.ui.settings.scale_error = Some(e),
        }
    }
    if let Some(error) = &c.ui.settings.scale_error {
        ui.label(RichText::new(error).size(12.0).color(theme.danger));
    }
}

fn panorama_control(ui: &mut egui::Ui, c: &mut Ctx<'_>) {
    let theme = c.theme.clone();
    let dir = c.state.data.dirs.custom_panorama_dir();
    ui.add_space(8.0);

    if secondary_button(ui, &theme, "Choose all four images…").clicked() {
        let files = pick_pngs("Choose four panorama faces in order");
        if files.len() != 4 {
            c.toasts.push(
                ToastKind::Error,
                "Choose exactly four PNG images, one for each panorama face.",
            );
        } else {
            let sources: [PathBuf; 4] = files.try_into().unwrap();
            match panorama::import_custom_panorama(&sources, &dir) {
                Ok((w, h)) => {
                    c.state.data.settings.custom_panorama_rev =
                        c.state.data.settings.custom_panorama_rev.wrapping_add(1);
                    c.toasts.push(
                        ToastKind::Success,
                        format!("Imported a {w}x{h} four-face panorama."),
                    );
                }
                Err(e) => c.toasts.push(ToastKind::Error, e),
            }
        }
    }

    ui.add_space(4.0);
    ui.horizontal_wrapped(|ui| {
        for i in 0..4 {
            ui.vertical(|ui| {
                ui.set_max_width(120.0);
                let name = c.ui.settings.panorama_sources[i]
                    .as_ref()
                    .and_then(|p| p.file_name())
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "Not chosen".to_string());
                ui.add(
                    egui::Label::new(
                        RichText::new(format!("Face {i}: {name}"))
                            .size(12.0)
                            .color(theme.text_dim),
                    )
                    .truncate(),
                );
                if secondary_button(ui, &theme, "Choose…").clicked()
                    && let Some(path) = pick_png(&format!("Choose panorama face {i}"))
                {
                    c.ui.settings.panorama_sources[i] = Some(path);
                }
            });
        }
    });
    if c.ui.settings.panorama_sources.iter().all(Option::is_some) {
        let sources: Vec<PathBuf> =
            c.ui.settings
                .panorama_sources
                .iter()
                .filter_map(|p| p.clone())
                .collect();
        if ui.button("Import these four faces").clicked() {
            let array: [PathBuf; 4] = sources.try_into().unwrap();
            match panorama::import_custom_panorama(&array, &dir) {
                Ok((w, h)) => {
                    c.state.data.settings.custom_panorama_rev =
                        c.state.data.settings.custom_panorama_rev.wrapping_add(1);
                    c.ui.settings.panorama_sources = [None, None, None, None];
                    c.toasts.push(
                        ToastKind::Success,
                        format!("Imported a {w}x{h} four-face panorama."),
                    );
                }
                Err(e) => c.toasts.push(ToastKind::Error, e),
            }
        }
    }

    c.textures.request(
        SETTINGS_PANORAMA_KEY,
        ImageSource::CustomPanorama(dir.clone()),
        512,
    );
    if let Some(tex) = c.textures.get(SETTINGS_PANORAMA_KEY) {
        let (rect, _) = ui.allocate_exact_size(vec2(320.0, 80.0), Sense::hover());
        ui.painter().image(
            tex.id(),
            rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
    } else if c.textures.has_failed(SETTINGS_PANORAMA_KEY) {
        caption(ui, &theme, "No complete four-face panorama imported yet.");
    } else {
        caption(ui, &theme, "Loading panorama preview…");
    }
    if ui.button("Remove imported panorama").clicked() {
        if let Err(e) = panorama::clear_custom_panorama(&dir) {
            c.toasts.push(ToastKind::Error, e);
        } else {
            c.state.data.settings.custom_panorama_rev =
                c.state.data.settings.custom_panorama_rev.wrapping_add(1);
            c.toasts.push(ToastKind::Info, "Custom panorama removed.");
        }
    }
}

fn folder_control(ui: &mut egui::Ui, c: &mut Ctx<'_>, theme: &Theme, path: std::path::PathBuf) {
    if secondary_button(ui, theme, "Open").clicked()
        && let Err(e) = open_folder(&path)
    {
        c.toasts
            .push(ToastKind::Error, format!("Could not open the folder: {e}"));
    }
    ui.add(
        egui::Label::new(
            RichText::new(path.display().to_string())
                .size(12.5)
                .color(theme.text_faint),
        )
        .truncate(),
    );
}
