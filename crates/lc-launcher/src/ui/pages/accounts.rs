//! Accounts: offline profiles that work today, Microsoft sign-in that does not yet.

use egui::{RichText, vec2};

use super::{Ctx, caption, confirm_modal, label};
use crate::model::account::AccountRecord;
use crate::ui::icons::paint_letter_tile;
use crate::ui::toasts::ToastKind;
use crate::ui::widgets::{
    accent_button, card, chip, column, cta_button, glass_panel, page_header, primary_button,
    section_title, small_button,
};

/// Accounts page state.
#[derive(Default)]
pub struct State {
    new_offline: String,
    error: Option<String>,
    remove: Option<String>,
}

/// Draw the Accounts page.
pub fn show(ui: &mut egui::Ui, c: &mut Ctx<'_>) {
    page_header(
        ui,
        c.theme,
        "Accounts",
        "Switch between accounts. Offline profiles work today; Microsoft sign-in is not implemented yet.",
    );
    let wide = ui.available_width() > 900.0;
    let list_width = if wide {
        ui.available_width() * 0.52
    } else {
        ui.available_width()
    };

    let draw_list = |ui: &mut egui::Ui, c: &mut Ctx<'_>| {
        section_title(ui, c.theme, "Your accounts");
        let accounts = c.state.data.accounts.accounts.clone();
        let selected = c.state.data.selected_account().map(|a| a.id().to_string());
        let mut pick = None;
        let mut remove = None;
        if accounts.is_empty() {
            glass_panel(ui, c.theme, |ui| {
                ui.set_width(ui.available_width());
                ui.label(
                    RichText::new("No accounts yet")
                        .size(17.0)
                        .strong()
                        .color(c.theme.text),
                );
                caption(
                    ui,
                    c.theme,
                    "Add an offline profile on the right to get started.",
                );
            });
        }
        egui::ScrollArea::vertical()
            .id_salt("accounts_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for a in &accounts {
                    let is_selected = selected.as_deref() == Some(a.id());
                    let resp = card(
                        ui,
                        c.theme,
                        vec2(ui.available_width(), 112.0),
                        is_selected,
                        |ui| {
                            ui.horizontal(|ui| {
                                let (r, _) =
                                    ui.allocate_exact_size(vec2(48.0, 48.0), egui::Sense::hover());
                                paint_letter_tile(ui.painter(), r, a.username(), a.id());
                                ui.vertical(|ui| {
                                    ui.label(
                                        RichText::new(a.username())
                                            .size(17.0)
                                            .strong()
                                            .color(c.theme.text),
                                    );
                                    ui.horizontal(|ui| {
                                        chip(
                                            ui,
                                            a.status_label(),
                                            if a.can_launch() {
                                                c.theme.accent_hi
                                            } else {
                                                c.theme.warn
                                            },
                                        );
                                        if is_selected {
                                            chip(ui, "Active", c.theme.ok);
                                        }
                                    });
                                    caption(
                                        ui,
                                        c.theme,
                                        &format!(
                                            "Added {}",
                                            crate::util::relative_time(
                                                crate::util::now_secs(),
                                                a.added()
                                            )
                                        ),
                                    );
                                });
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if small_button(ui, c.theme, "Remove", true).clicked() {
                                            remove = Some(a.id().to_string());
                                        }
                                        if is_selected {
                                            accent_button(ui, c.theme, "Active");
                                        } else if small_button(ui, c.theme, "Use", false).clicked()
                                        {
                                            pick = Some(a.id().to_string());
                                        }
                                    },
                                );
                            });
                        },
                    );
                    if resp.clicked() && pick.is_none() && !is_selected && a.can_launch() {
                        pick = Some(a.id().to_string());
                    }
                    ui.add_space(2.0);
                }
            });
        if let Some(id) = pick {
            c.state.data.select_account(&id);
        }
        if let Some(id) = remove {
            c.ui.accounts.remove = Some(id);
        }
    };

    let draw_add = |ui: &mut egui::Ui, c: &mut Ctx<'_>| {
        section_title(ui, c.theme, "Microsoft account");
        glass_panel(ui, c.theme, |ui| {
            ui.set_width(ui.available_width());
            ui.label(
                RichText::new("Sign in with Microsoft")
                    .size(17.0)
                    .strong()
                    .color(c.theme.text),
            );
            caption(
                ui,
                c.theme,
                "Not available yet. Microsoft authentication is not implemented, so no sign-in is attempted and no tokens are stored.",
            );
            ui.add_space(6.0);
            if primary_button(
                ui,
                c.theme,
                "Sign in with Microsoft",
                vec2(280.0, 46.0),
                false,
            )
            .clicked()
            {
                c.toasts.push(
                    ToastKind::Warning,
                    "Microsoft sign-in is not implemented yet. Nothing was sent anywhere.",
                );
            }
        });
        ui.add_space(14.0);
        section_title(ui, c.theme, "Offline profile");
        glass_panel(ui, c.theme, |ui| {
            ui.set_width(ui.available_width());
            caption(
                ui,
                c.theme,
                "Offline profiles only work on servers that allow offline mode (online-mode=false). They never bypass a server's authentication.",
            );
            ui.add_space(6.0);
            label(ui, c.theme, "Username");
            let mut submit = false;
            ui.horizontal(|ui| {
                let resp = ui.add(
                    egui::TextEdit::singleline(&mut c.ui.accounts.new_offline)
                        .hint_text("3 to 16 letters, digits or _")
                        .desired_width(240.0),
                );
                if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    submit = true;
                }
                if cta_button(ui, c.theme, "Add profile").clicked() {
                    submit = true;
                }
            });
            if let Some(err) = &c.ui.accounts.error {
                ui.label(RichText::new(err).color(c.theme.danger));
            }
            if submit {
                let name = c.ui.accounts.new_offline.clone();
                match c.state.data.accounts.add_offline(&name) {
                    Ok(id) => {
                        c.ui.accounts.new_offline.clear();
                        c.ui.accounts.error = None;
                        if c.state.data.selected_account().map(|a| a.id()) != Some(id.as_str())
                            && c.state.data.accounts.accounts.len() == 1
                        {
                            c.state.data.select_account(&id);
                        }
                        c.toasts.push(ToastKind::Success, "Offline profile added.");
                    }
                    Err(e) => c.ui.accounts.error = Some(e.to_string()),
                }
            }
        });
    };

    if wide {
        ui.horizontal_top(|ui| {
            column(ui, vec2(list_width - 12.0, ui.available_height()), |ui| {
                draw_list(ui, c)
            });
            ui.add_space(12.0);
            column(
                ui,
                vec2(ui.available_width(), ui.available_height()),
                |ui| draw_add(ui, c),
            );
        });
    } else {
        draw_add(ui, c);
        ui.add_space(12.0);
        draw_list(ui, c);
    }

    if let Some(id) = c.ui.accounts.remove.clone() {
        let name = c
            .state
            .data
            .accounts
            .find(&id)
            .map(AccountRecord::username)
            .unwrap_or_default()
            .to_string();
        match confirm_modal(
            c.egui,
            c.theme,
            "remove_account",
            "Remove account?",
            &format!("\"{name}\" will be removed from the launcher."),
            "Remove",
        ) {
            Some(true) => {
                c.state.data.accounts.remove(&id);
                c.ui.accounts.remove = None;
                c.toasts.push(ToastKind::Info, "Account removed.");
            }
            Some(false) => c.ui.accounts.remove = None,
            None => {}
        }
    }
}
