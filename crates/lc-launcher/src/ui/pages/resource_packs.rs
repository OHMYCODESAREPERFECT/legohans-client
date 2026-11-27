//! Resource packs: import, remove, preview, enable/disable and order.

use std::collections::HashMap;
use std::path::Path;

use egui::{Color32, Rect, RichText, pos2, vec2};

use super::{Ctx, caption};
use crate::model::resource_pack::{self, PackInfo, PackKind};
use crate::ui::icons::paint_letter_tile;
use crate::ui::toasts::ToastKind;
use crate::ui::widgets::{
    arrow_button, card, chip, column, cta_button, danger_button, glass_panel, page_header,
    secondary_button, section_title, toggle,
};
use crate::util::{format_bytes, open_folder};

/// Resource packs page state.
#[derive(Default)]
pub struct State {
    import_path: String,
    selected: Option<String>,
    info: HashMap<String, Result<PackInfo, String>>,
    remove: Option<String>,
}

fn import_from(c: &mut Ctx<'_>, path: &Path) {
    let dir = c.state.data.dirs.resource_packs_dir();
    match resource_pack::import(path, &dir) {
        Ok(name) => {
            c.state.data.packs.sync(&dir);
            c.ui.packs.info.remove(&name);
            c.ui.packs.selected = Some(name.clone());
            c.toasts.push(
                ToastKind::Success,
                format!("Imported \"{name}\". It is disabled until you turn it on."),
            );
        }
        Err(e) => c.toasts.push(ToastKind::Error, e.to_string()),
    }
}

fn info_for<'a>(c: &'a mut Ctx<'_>, name: &str) -> &'a Result<PackInfo, String> {
    if !c.ui.packs.info.contains_key(name) {
        let path = c.state.data.dirs.resource_packs_dir().join(name);
        let result = resource_pack::inspect(&path).map_err(|e| e.to_string());
        c.ui.packs.info.insert(name.to_string(), result);
    }
    &c.ui.packs.info[name]
}

fn paint_pack_icon(
    c: &mut Ctx<'_>,
    ui: &egui::Ui,
    rect: Rect,
    name: &str,
    info: Option<&PackInfo>,
) {
    let key = format!("pack:{name}");
    if let Some(bytes) = info.and_then(|i| i.icon_png.as_ref()) {
        c.textures.load_bytes(&key, bytes, 128);
    }
    match c.textures.get(&key) {
        Some(tex) => {
            ui.painter().image(
                tex.id(),
                rect,
                Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        }
        None => paint_letter_tile(ui.painter(), rect, name, name),
    }
}

/// Draw the Resource Packs page.
pub fn show(ui: &mut egui::Ui, c: &mut Ctx<'_>) {
    page_header(
        ui,
        c.theme,
        "Resource Packs",
        "Import packs, turn them on or off and set their order. The top pack wins.",
    );

    let dropped: Vec<_> = c.egui.input(|i| i.raw.dropped_files.clone());
    for file in dropped {
        if let Some(path) = file.path {
            import_from(c, &path);
        }
    }

    let dir = c.state.data.dirs.resource_packs_dir();
    ui.horizontal(|ui| {
        let resp = ui.add(
            egui::TextEdit::singleline(&mut c.ui.packs.import_path)
                .hint_text("Path to a .zip pack or a pack folder, or drop it onto the window")
                .desired_width((ui.available_width() - 330.0).clamp(200.0, 560.0)),
        );
        let enter = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        if cta_button(ui, c.theme, "Import").clicked() || enter {
            let text = c.ui.packs.import_path.trim().trim_matches('"').to_string();
            if text.is_empty() {
                c.toasts.push(
                    ToastKind::Warning,
                    "Enter or drop the path of a pack first.",
                );
            } else {
                import_from(c, Path::new(&text));
                c.ui.packs.import_path.clear();
            }
        }
        if secondary_button(ui, c.theme, "Open packs folder").clicked()
            && let Err(e) = open_folder(&dir)
        {
            c.toasts
                .push(ToastKind::Error, format!("Could not open the folder: {e}"));
        }
    });
    ui.add_space(6.0);

    if c.egui.input(|i| !i.raw.hovered_files.is_empty()) {
        let r = ui.ctx().content_rect();
        let p = ui.ctx().layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            egui::Id::new("drop"),
        ));
        p.rect_filled(r, 0.0, Color32::from_rgba_unmultiplied(10, 6, 20, 200));
        p.text(
            r.center(),
            egui::Align2::CENTER_CENTER,
            "Drop a resource pack (.zip or folder) to import it",
            egui::FontId::proportional(26.0),
            c.theme.accent_hi,
        );
    }

    let records = c.state.data.packs.records.clone();
    if c.ui.packs.selected.is_none()
        || !records
            .iter()
            .any(|r| Some(&r.file_name) == c.ui.packs.selected.as_ref())
    {
        c.ui.packs.selected = records.first().map(|r| r.file_name.clone());
    }

    let wide = ui.available_width() > 860.0;
    let list_w = if wide {
        ui.available_width() * 0.56
    } else {
        ui.available_width()
    };

    let draw_list = |ui: &mut egui::Ui, c: &mut Ctx<'_>| {
        section_title(ui, c.theme, &format!("Installed ({})", records.len()));
        if records.is_empty() {
            glass_panel(ui, c.theme, |ui| {
                ui.set_width(ui.available_width());
                ui.label(
                    RichText::new("No resource packs yet")
                        .size(17.0)
                        .strong()
                        .color(c.theme.text),
                );
                caption(
                    ui,
                    c.theme,
                    "Import a .zip or folder that contains pack.mcmeta.",
                );
            });
            return;
        }
        let mut move_up = None;
        let mut move_down = None;
        let mut toggled: Option<(String, bool)> = None;
        let mut select = None;
        egui::ScrollArea::vertical()
            .id_salt("packs_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for (i, rec) in records.iter().enumerate() {
                    let info = info_for(c, &rec.file_name).clone();
                    let is_sel = c.ui.packs.selected.as_deref() == Some(rec.file_name.as_str());
                    let resp = card(
                        ui,
                        c.theme,
                        vec2(ui.available_width(), 84.0),
                        is_sel,
                        |ui| {
                            ui.horizontal(|ui| {
                                let (r, _) =
                                    ui.allocate_exact_size(vec2(52.0, 52.0), egui::Sense::hover());
                                paint_pack_icon(c, ui, r, &rec.file_name, info.as_ref().ok());
                                ui.vertical(|ui| {
                                    ui.set_width((ui.available_width() - 150.0).max(80.0));
                                    ui.add(
                                        egui::Label::new(
                                            RichText::new(&rec.file_name)
                                                .size(16.0)
                                                .strong()
                                                .color(c.theme.text),
                                        )
                                        .truncate(),
                                    );
                                    match &info {
                                        Ok(i) => {
                                            ui.add(
                                                egui::Label::new(
                                                    RichText::new(&i.description)
                                                        .size(13.0)
                                                        .color(c.theme.text_dim),
                                                )
                                                .truncate(),
                                            );
                                        }
                                        Err(e) => {
                                            ui.add(
                                                egui::Label::new(
                                                    RichText::new(e)
                                                        .size(13.0)
                                                        .color(c.theme.danger),
                                                )
                                                .truncate(),
                                            );
                                        }
                                    }
                                });
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        let mut on = rec.enabled;
                                        if toggle(ui, c.theme, &mut on).clicked() {
                                            toggled = Some((rec.file_name.clone(), on));
                                        }
                                        ui.vertical(|ui| {
                                            ui.spacing_mut().item_spacing.y = 0.0;
                                            ui.add_enabled_ui(i > 0, |ui| {
                                                if arrow_button(ui, c.theme, true)
                                                    .on_hover_text("Higher priority")
                                                    .clicked()
                                                {
                                                    move_up = Some(i);
                                                }
                                            });
                                            ui.add_enabled_ui(i + 1 < records.len(), |ui| {
                                                if arrow_button(ui, c.theme, false)
                                                    .on_hover_text("Lower priority")
                                                    .clicked()
                                                {
                                                    move_down = Some(i);
                                                }
                                            });
                                        });
                                    },
                                );
                            });
                        },
                    );
                    if resp.clicked() {
                        select = Some(rec.file_name.clone());
                    }
                    ui.add_space(2.0);
                }
            });
        if let Some(s) = select {
            c.ui.packs.selected = Some(s);
        }
        if let Some((name, on)) = toggled {
            c.state.data.packs.set_enabled(&name, on);
        }
        if let Some(i) = move_up {
            c.state.data.packs.move_up(i);
        }
        if let Some(i) = move_down {
            c.state.data.packs.move_down(i);
        }
    };

    let draw_preview = |ui: &mut egui::Ui, c: &mut Ctx<'_>| {
        section_title(ui, c.theme, "Preview");
        let Some(name) = c.ui.packs.selected.clone() else {
            glass_panel(ui, c.theme, |ui| {
                ui.set_width(ui.available_width());
                caption(ui, c.theme, "Select a pack to see its details.");
            });
            return;
        };
        let info = info_for(c, &name).clone();
        let enabled = c
            .state
            .data
            .packs
            .records
            .iter()
            .find(|r| r.file_name == name)
            .is_some_and(|r| r.enabled);
        glass_panel(ui, c.theme, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(vec2(96.0, 96.0), egui::Sense::hover());
                paint_pack_icon(c, ui, r, &name, info.as_ref().ok());
                ui.vertical(|ui| {
                    ui.add(
                        egui::Label::new(
                            RichText::new(&name).size(20.0).strong().color(c.theme.text),
                        )
                        .truncate(),
                    );
                    ui.horizontal(|ui| {
                        chip(
                            ui,
                            if enabled { "Enabled" } else { "Disabled" },
                            if enabled {
                                c.theme.ok
                            } else {
                                c.theme.text_dim
                            },
                        );
                        if let Ok(i) = &info {
                            chip(
                                ui,
                                if i.kind == PackKind::Zip {
                                    "Zip file"
                                } else {
                                    "Folder"
                                },
                                c.theme.accent_hi,
                            );
                        }
                    });
                });
            });
            ui.add_space(6.0);
            match &info {
                Ok(i) => {
                    if !i.description.is_empty() {
                        ui.label(RichText::new(&i.description).size(14.5).color(c.theme.text));
                    }
                    caption(
                        ui,
                        c.theme,
                        &format!(
                            "Pack format: {}",
                            i.format_label.as_deref().unwrap_or("not declared")
                        ),
                    );
                    caption(
                        ui,
                        c.theme,
                        &format!("Size: {}", format_bytes(i.size_bytes)),
                    );
                }
                Err(e) => {
                    ui.label(RichText::new(e).color(c.theme.danger));
                }
            }
            ui.add_space(8.0);
            if danger_button(ui, c.theme, "Remove pack").clicked() {
                c.ui.packs.remove = Some(name.clone());
            }
            ui.add_space(6.0);
            caption(
                ui,
                c.theme,
                "Packs are stored and ordered here. They are applied once the Rust game client exists.",
            );
        });
    };

    if wide {
        ui.horizontal_top(|ui| {
            column(ui, vec2(list_w - 12.0, ui.available_height()), |ui| {
                draw_list(ui, c)
            });
            ui.add_space(12.0);
            column(
                ui,
                vec2(ui.available_width(), ui.available_height()),
                |ui| draw_preview(ui, c),
            );
        });
    } else {
        draw_preview(ui, c);
        ui.add_space(12.0);
        draw_list(ui, c);
    }

    if let Some(name) = c.ui.packs.remove.clone() {
        match super::confirm_modal(
            c.egui,
            c.theme,
            "remove_pack",
            "Remove resource pack?",
            &format!(
                "\"{name}\" will be deleted from the launcher's packs folder. Your original file is not touched."
            ),
            "Remove",
        ) {
            Some(true) => {
                match resource_pack::remove(&c.state.data.dirs.resource_packs_dir(), &name) {
                    Ok(()) => {
                        c.state.data.packs.forget(&name);
                        c.ui.packs.info.remove(&name);
                        c.textures.evict(&format!("pack:{name}"));
                        c.toasts.push(ToastKind::Info, "Resource pack removed.");
                    }
                    Err(e) => c
                        .toasts
                        .push(ToastKind::Error, format!("Could not remove the pack: {e}")),
                }
                c.ui.packs.remove = None;
            }
            Some(false) => c.ui.packs.remove = None,
            None => {}
        }
    }
    let _ = Rect::NOTHING;
}
