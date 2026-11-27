//! Settings: categorized, searchable launcher settings.

use egui::{Margin, RichText, Sense, Stroke, vec2};

use super::{Ctx, caption, confirm_modal};
use crate::model::settings::{
    Accent, AfterLaunch, Category, HomeBackground, SettingId, SettingMeta, Settings,
    UI_SCALE_RANGE, search,
};
use crate::services::panorama::find_client_jar;
use crate::ui::theme::Theme;
use crate::ui::toasts::ToastKind;
use crate::ui::widgets::{
    glass_frame, page_header, secondary_button, section_title, toggle, wide_button,
};
use crate::util::open_folder;

/// Settings page state.
#[derive(Default)]
pub struct State {
    query: String,
    confirm_reset: bool,
}

/// Draw the Settings page.
pub fn show(ui: &mut egui::Ui, c: &mut Ctx<'_>) {
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
                c.toasts
                    .push(ToastKind::Info, "Settings reset to defaults.");
            }
            Some(false) => c.ui.settings.confirm_reset = false,
            None => {}
        }
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
    let dirs = c.state.data.dirs.clone();
    let s = &mut c.state.data.settings;
    match id {
        SettingId::Accent => {
            for accent in Accent::ALL.iter().rev() {
                let color = Theme::new(*accent, true).accent;
                let (rect, resp) = ui.allocate_exact_size(vec2(28.0, 28.0), Sense::click());
                let on = s.accent == *accent;
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
                                theme.border
                            },
                        ),
                    );
                }
                if resp.on_hover_text(accent.name()).clicked() {
                    s.accent = *accent;
                }
            }
        }
        SettingId::UiScale => {
            ui.add(
                egui::Slider::new(&mut s.ui_scale, UI_SCALE_RANGE)
                    .step_by(0.05)
                    .custom_formatter(|v, _| format!("{:.0}%", v * 100.0)),
            );
        }
        SettingId::Animations => {
            toggle(ui, &theme, &mut s.animations);
        }
        SettingId::CompactSidebar => {
            toggle(ui, &theme, &mut s.compact_sidebar);
        }
        SettingId::ShowSplash => {
            toggle(ui, &theme, &mut s.show_splash);
        }
        SettingId::CheckUpdates => {
            toggle(ui, &theme, &mut s.check_updates);
        }
        SettingId::HomeBackground => {
            egui::ComboBox::from_id_salt("home_bg")
                .selected_text(s.home_background.name())
                .show_ui(ui, |ui| {
                    for b in HomeBackground::ALL {
                        ui.selectable_value(&mut s.home_background, b, b.name());
                    }
                });
        }
        SettingId::AfterLaunch => {
            egui::ComboBox::from_id_salt("after_launch")
                .selected_text(s.after_launch.name())
                .show_ui(ui, |ui| {
                    for a in AfterLaunch::ALL {
                        ui.selectable_value(&mut s.after_launch, a, a.name());
                    }
                });
        }
        SettingId::MinecraftDir => {
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
        SettingId::DataFolder => folder_control(ui, c, &theme, dirs.root().to_path_buf()),
        SettingId::ScreenshotsFolder => folder_control(ui, c, &theme, dirs.screenshots_dir()),
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
