//! Installations: visual profile cards with create, edit and delete.

use egui::{Id, Label, Rect, RichText, Sense, Stroke, StrokeKind, vec2};

use super::{Ctx, caption, confirm_modal, label, last_played_text};
use crate::model::profile::{Profile, ProfileIcon, SUPPORTED_VERSIONS};
use crate::ui::icons::paint_block_icon;
use crate::ui::toasts::ToastKind;
use crate::ui::widgets::{
    accent_button, card, chip, cta_button, modal_frame, page_header, secondary_button, small_button,
};
use crate::util::{now_secs, open_folder};

/// The profile editor dialog state.
pub struct Editor {
    profile: Profile,
    is_new: bool,
    custom_dir: bool,
    error: Option<String>,
}

/// Installations page state.
#[derive(Default)]
pub struct State {
    editor: Option<Editor>,
    delete: Option<String>,
}

#[derive(Default)]
struct Actions {
    select: Option<String>,
    edit: Option<String>,
    duplicate: Option<String>,
    delete: Option<String>,
    folder: Option<String>,
}

/// Draw the Installations page.
pub fn show(ui: &mut egui::Ui, c: &mut Ctx<'_>) {
    page_header(
        ui,
        c.theme,
        "Installations",
        "Profiles for the Rust client. Each one has its own game folder and window settings.",
    );
    ui.horizontal(|ui| {
        if cta_button(ui, c.theme, "+  New installation").clicked() {
            c.ui.installs.editor = Some(Editor {
                profile: Profile::new("New installation"),
                is_new: true,
                custom_dir: false,
                error: None,
            });
        }
        caption(
            ui,
            c.theme,
            "Only Minecraft 1.21.11 is supported. The client targets exactly that version.",
        );
    });
    ui.add_space(8.0);

    let profiles = c.state.data.profiles.profiles.clone();
    let selected = c.state.data.selected_profile().map(|p| p.id.clone());
    let can_delete = profiles.len() > 1;
    let now = now_secs();
    let mut actions = Actions::default();

    egui::ScrollArea::vertical()
        .id_salt("installations_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let gap = 16.0;
            let avail = ui.available_width();
            let cols = (((avail + gap) / (380.0 + gap)).floor() as usize).max(1);
            let width = ((avail - gap * (cols as f32 - 1.0)) / cols as f32).floor();
            ui.spacing_mut().item_spacing = vec2(gap, gap);
            ui.horizontal_wrapped(|ui| {
                for p in &profiles {
                    let is_selected = selected.as_deref() == Some(p.id.as_str());
                    let resp = card(ui, c.theme, vec2(width, 200.0), is_selected, |ui| {
                        ui.spacing_mut().item_spacing = vec2(8.0, 6.0);
                        ui.horizontal(|ui| {
                            let (r, _) = ui.allocate_exact_size(vec2(62.0, 62.0), Sense::hover());
                            paint_block_icon(ui.painter(), r, p.icon, c.theme.accent_alpha(60));
                            ui.vertical(|ui| {
                                ui.add(
                                    Label::new(
                                        RichText::new(&p.name)
                                            .size(18.0)
                                            .strong()
                                            .color(c.theme.text),
                                    )
                                    .truncate(),
                                );
                                ui.horizontal(|ui| {
                                    chip(ui, &p.version, c.theme.accent_hi);
                                    if is_selected {
                                        chip(ui, "Selected", c.theme.ok);
                                    }
                                });
                            });
                        });
                        if !p.notes.trim().is_empty() {
                            ui.add(
                                Label::new(
                                    RichText::new(&p.notes).size(13.0).color(c.theme.text_dim),
                                )
                                .truncate(),
                            );
                        }
                        let window = if p.fullscreen {
                            "Fullscreen".to_string()
                        } else {
                            format!("{} \u{d7} {}", p.width, p.height)
                        };
                        caption(
                            ui,
                            c.theme,
                            &format!("{window}  ·  {}", last_played_text(p.last_played, now)),
                        );
                        ui.add(
                            Label::new(
                                RichText::new(
                                    p.resolved_game_dir(&c.state.data.dirs)
                                        .display()
                                        .to_string(),
                                )
                                .size(12.0)
                                .color(c.theme.text_faint),
                            )
                            .truncate(),
                        );
                        ui.add_space(2.0);
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 6.0;
                            if is_selected {
                                accent_button(ui, c.theme, "Selected");
                            } else if small_button(ui, c.theme, "Select", false).clicked() {
                                actions.select = Some(p.id.clone());
                            }
                            if small_button(ui, c.theme, "Edit", false).clicked() {
                                actions.edit = Some(p.id.clone());
                            }
                            if small_button(ui, c.theme, "Copy", false).clicked() {
                                actions.duplicate = Some(p.id.clone());
                            }
                            if small_button(ui, c.theme, "Folder", false).clicked() {
                                actions.folder = Some(p.id.clone());
                            }
                            ui.add_enabled_ui(can_delete, |ui| {
                                let r = small_button(ui, c.theme, "Delete", true);
                                if r.clicked() {
                                    actions.delete = Some(p.id.clone());
                                }
                                if !can_delete {
                                    r.on_disabled_hover_text(
                                        "The last installation cannot be deleted.",
                                    );
                                }
                            });
                        });
                    });
                    if resp.clicked() && actions.select.is_none() && !is_selected {
                        actions.select = Some(p.id.clone());
                    }
                }
            });
        });

    apply_actions(c, actions);
    editor_modal(c);
    delete_modal(c);
}

fn apply_actions(c: &mut Ctx<'_>, a: Actions) {
    if let Some(id) = a.select {
        c.state.data.select_profile(&id);
    }
    if let Some(id) = a.edit
        && let Some(p) = c.state.data.profiles.find(&id).cloned()
    {
        let custom_dir = p.game_dir.is_some();
        c.ui.installs.editor = Some(Editor {
            profile: p,
            is_new: false,
            custom_dir,
            error: None,
        });
    }
    if let Some(id) = a.duplicate
        && c.state.data.profiles.duplicate(&id).is_some()
    {
        c.toasts
            .push(ToastKind::Success, "Installation duplicated.");
    }
    if let Some(id) = a.folder
        && let Some(p) = c.state.data.profiles.find(&id)
    {
        let dir = p.resolved_game_dir(&c.state.data.dirs);
        if let Err(e) = open_folder(&dir) {
            c.toasts.push(
                ToastKind::Error,
                format!("Could not open {}: {e}", dir.display()),
            );
        }
    }
    if let Some(id) = a.delete {
        c.ui.installs.delete = Some(id);
    }
}

fn delete_modal(c: &mut Ctx<'_>) {
    let Some(id) = c.ui.installs.delete.clone() else {
        return;
    };
    let name = c
        .state
        .data
        .profiles
        .find(&id)
        .map(|p| p.name.clone())
        .unwrap_or_default();
    match confirm_modal(
        c.egui,
        c.theme,
        "delete_profile",
        "Delete installation?",
        &format!("\"{name}\" will be removed from the launcher. Its game folder stays on disk."),
        "Delete",
    ) {
        Some(true) => {
            if c.state.data.profiles.delete(&id) {
                c.toasts.push(
                    ToastKind::Info,
                    "Installation deleted. Its folder was left on disk.",
                );
            }
            c.ui.installs.delete = None;
        }
        Some(false) => c.ui.installs.delete = None,
        None => {}
    }
}

fn editor_modal(c: &mut Ctx<'_>) {
    let Some(editor) = c.ui.installs.editor.as_mut() else {
        return;
    };
    let theme = c.theme;
    let default_dir = c
        .state
        .data
        .dirs
        .instance_dir(&editor.profile.id)
        .display()
        .to_string();
    let mut save = false;
    let mut cancel = false;
    let modal = egui::Modal::new(Id::new("profile_editor"))
        .frame(modal_frame(theme))
        .show(c.egui, |ui| {
            ui.set_width(480.0);
            let title = if editor.is_new {
                "New installation"
            } else {
                "Edit installation"
            };
            ui.label(RichText::new(title).size(22.0).strong().color(theme.text));
            ui.add_space(6.0);

            label(ui, theme, "Name");
            ui.add(
                egui::TextEdit::singleline(&mut editor.profile.name).desired_width(f32::INFINITY),
            );

            label(ui, theme, "Game version");
            egui::ComboBox::from_id_salt("profile_version")
                .selected_text(editor.profile.version.clone())
                .show_ui(ui, |ui| {
                    for v in SUPPORTED_VERSIONS {
                        ui.selectable_value(&mut editor.profile.version, (*v).to_string(), *v);
                    }
                });

            label(ui, theme, "Icon");
            ui.horizontal(|ui| {
                for icon in ProfileIcon::ALL {
                    let (rect, resp) = ui.allocate_exact_size(vec2(46.0, 46.0), Sense::click());
                    let on = editor.profile.icon == icon;
                    if on || resp.hovered() {
                        ui.painter().rect_stroke(
                            rect,
                            egui::CornerRadius::same(10),
                            Stroke::new(1.5, if on { theme.accent_hi } else { theme.border }),
                            StrokeKind::Inside,
                        );
                    }
                    paint_block_icon(
                        ui.painter(),
                        Rect::from_center_size(rect.center(), vec2(32.0, 32.0)),
                        icon,
                        theme.accent_alpha(if on { 70 } else { 0 }),
                    );
                    if resp.on_hover_text(icon.name()).clicked() {
                        editor.profile.icon = icon;
                    }
                }
            });

            label(ui, theme, "Notes");
            ui.add(
                egui::TextEdit::multiline(&mut editor.profile.notes)
                    .desired_rows(2)
                    .desired_width(f32::INFINITY),
            );

            label(ui, theme, "Window");
            ui.horizontal(|ui| {
                ui.add(
                    egui::DragValue::new(&mut editor.profile.width)
                        .range(320..=7680)
                        .suffix(" px"),
                );
                ui.label("\u{d7}");
                ui.add(
                    egui::DragValue::new(&mut editor.profile.height)
                        .range(240..=4320)
                        .suffix(" px"),
                );
                ui.checkbox(&mut editor.profile.fullscreen, "Start fullscreen");
            });

            label(ui, theme, "Game folder");
            ui.checkbox(&mut editor.custom_dir, "Use a custom folder");
            if editor.custom_dir {
                let dir = editor.profile.game_dir.get_or_insert_with(String::new);
                ui.add(
                    egui::TextEdit::singleline(dir)
                        .hint_text("Full path to the game folder")
                        .desired_width(f32::INFINITY),
                );
            } else {
                caption(ui, theme, &format!("Default: {default_dir}"));
            }

            if let Some(err) = &editor.error {
                ui.add_space(4.0);
                ui.label(RichText::new(err).color(theme.danger));
            }
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if cta_button(ui, theme, "Save").clicked() {
                    save = true;
                }
                if secondary_button(ui, theme, "Cancel").clicked() {
                    cancel = true;
                }
            });
        });
    if modal.should_close() {
        cancel = true;
    }

    if save {
        let mut profile = editor.profile.clone();
        if !editor.custom_dir {
            profile.game_dir = None;
        }
        let is_new = editor.is_new;
        let result = if is_new {
            c.state.data.profiles.add(profile).map(|id| {
                c.state.data.select_profile(&id);
            })
        } else {
            c.state.data.profiles.update(profile)
        };
        match result {
            Ok(()) => {
                c.ui.installs.editor = None;
                c.toasts.push(
                    ToastKind::Success,
                    if is_new {
                        "Installation created."
                    } else {
                        "Installation saved."
                    },
                );
            }
            Err(e) => {
                if let Some(ed) = c.ui.installs.editor.as_mut() {
                    ed.error = Some(e.to_string());
                }
            }
        }
    } else if cancel {
        c.ui.installs.editor = None;
    }
}
