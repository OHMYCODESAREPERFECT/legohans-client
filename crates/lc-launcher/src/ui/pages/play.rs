//! Play: choose an installation, account and optional server, then launch.

use egui::{RichText, vec2};

use super::{Ctx, backend_chip, caption, last_played_text, launch_and_report};
use crate::backend::BackendStatus;
use crate::ui::Page;
use crate::ui::icons::paint_block_icon;
use crate::ui::widgets::{
    card, chip, column, glass_panel, page_header, primary_button, secondary_button, section_title,
};
use crate::util::{now_secs, relative_time};

/// Draw the Play page.
pub fn show(ui: &mut egui::Ui, c: &mut Ctx<'_>) {
    page_header(
        ui,
        c.theme,
        "Play",
        "Pick an installation and an account, then launch.",
    );
    let wide = ui.available_width() > 860.0;
    let left_width = if wide {
        ui.available_width() * 0.5
    } else {
        ui.available_width()
    };

    let draw_left = |ui: &mut egui::Ui, c: &mut Ctx<'_>| {
        section_title(ui, c.theme, "Installation");
        let profiles = c.state.data.profiles.profiles.clone();
        let selected = c.state.data.selected_profile().map(|p| p.id.clone());
        let now = now_secs();
        let mut pick = None;
        egui::ScrollArea::vertical()
            .id_salt("play_profiles")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for p in &profiles {
                    let is_selected = selected.as_deref() == Some(p.id.as_str());
                    let resp = card(
                        ui,
                        c.theme,
                        vec2(ui.available_width(), 84.0),
                        is_selected,
                        |ui| {
                            ui.horizontal(|ui| {
                                let (r, _) =
                                    ui.allocate_exact_size(vec2(50.0, 50.0), egui::Sense::hover());
                                paint_block_icon(ui.painter(), r, p.icon, c.theme.accent_alpha(50));
                                ui.vertical(|ui| {
                                    ui.label(
                                        RichText::new(&p.name)
                                            .size(17.0)
                                            .strong()
                                            .color(c.theme.text),
                                    );
                                    ui.label(
                                        RichText::new(format!(
                                            "{}  ·  {}",
                                            p.version,
                                            last_played_text(p.last_played, now)
                                        ))
                                        .size(13.0)
                                        .color(c.theme.text_dim),
                                    );
                                });
                            });
                        },
                    );
                    if resp.clicked() {
                        pick = Some(p.id.clone());
                    }
                    ui.add_space(2.0);
                }
            });
        if let Some(id) = pick {
            c.state.data.select_profile(&id);
        }
    };

    let draw_right = |ui: &mut egui::Ui, c: &mut Ctx<'_>| {
        section_title(ui, c.theme, "Launch");
        glass_panel(ui, c.theme, |ui| {
            ui.set_width(ui.available_width());
            super::label(ui, c.theme, "Account");
            let accounts = c.state.data.accounts.accounts.clone();
            if accounts.is_empty() {
                caption(ui, c.theme, "No accounts yet.");
                if secondary_button(ui, c.theme, "Add an account").clicked() {
                    c.ui.next_page = Some(Page::Accounts);
                }
            } else {
                let current = c
                    .state
                    .data
                    .selected_account()
                    .map(|a| (a.id().to_string(), a.username().to_string()));
                let mut pick = None;
                egui::ComboBox::from_id_salt("play_account")
                    .width(ui.available_width().min(360.0))
                    .selected_text(current.as_ref().map(|(_, n)| n.clone()).unwrap_or_default())
                    .show_ui(ui, |ui| {
                        for a in &accounts {
                            let text = format!("{}  ({})", a.username(), a.status_label());
                            if ui
                                .selectable_label(
                                    current.as_ref().is_some_and(|(id, _)| id == a.id()),
                                    text,
                                )
                                .clicked()
                            {
                                pick = Some(a.id().to_string());
                            }
                        }
                    });
                if let Some(id) = pick {
                    c.state.data.select_account(&id);
                }
            }
            ui.add_space(8.0);
            super::label(ui, c.theme, "Join a server on launch (optional)");
            let servers = c.state.data.servers.servers.clone();
            let selected_name =
                c.ui.servers
                    .join_on_launch
                    .as_ref()
                    .and_then(|id| servers.iter().find(|s| &s.id == id))
                    .map(|s| s.name.clone())
                    .unwrap_or_else(|| "None, just start the game".to_string());
            egui::ComboBox::from_id_salt("play_server")
                .width(ui.available_width().min(360.0))
                .selected_text(selected_name)
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(
                            c.ui.servers.join_on_launch.is_none(),
                            "None, just start the game",
                        )
                        .clicked()
                    {
                        c.ui.servers.join_on_launch = None;
                    }
                    for s in &servers {
                        let on = c.ui.servers.join_on_launch.as_deref() == Some(s.id.as_str());
                        if ui
                            .selectable_label(on, format!("{}  ({})", s.name, s.address))
                            .clicked()
                        {
                            c.ui.servers.join_on_launch = Some(s.id.clone());
                        }
                    }
                });
            ui.add_space(14.0);
            let ready = matches!(c.state.backend_status(), BackendStatus::Ready);
            ui.horizontal(|ui| {
                if primary_button(ui, c.theme, "LAUNCH", vec2(220.0, 58.0), ready).clicked() {
                    let server = c.ui.servers.join_on_launch.clone();
                    launch_and_report(c, server.as_deref());
                }
                ui.vertical(|ui| {
                    ui.add_space(4.0);
                    backend_chip(ui, c);
                });
            });
            ui.add_space(8.0);
            match c.state.backend_status() {
                BackendStatus::Ready => caption(
                    ui,
                    c.theme,
                    "Launching starts the game immediately, with no extra steps.",
                ),
                BackendStatus::Unavailable(reason) => caption(
                    ui,
                    c.theme,
                    &format!(
                        "{reason} Pressing Launch will explain this instead of pretending to start."
                    ),
                ),
            }
            if let Some(log) = c.state.last_launch.clone() {
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    chip(
                        ui,
                        if log.success {
                            "Last attempt: started"
                        } else {
                            "Last attempt: not started"
                        },
                        if log.success {
                            c.theme.ok
                        } else {
                            c.theme.warn
                        },
                    );
                    caption(ui, c.theme, &relative_time(now_secs(), log.at));
                });
                caption(ui, c.theme, &log.message);
            }
        });
    };

    if wide {
        ui.horizontal_top(|ui| {
            column(ui, vec2(left_width - 12.0, ui.available_height()), |ui| {
                draw_left(ui, c)
            });
            ui.add_space(12.0);
            column(
                ui,
                vec2(ui.available_width(), ui.available_height()),
                |ui| draw_right(ui, c),
            );
        });
    } else {
        draw_right(ui, c);
        ui.add_space(12.0);
        draw_left(ui, c);
    }
}
