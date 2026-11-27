//! Servers: saved servers with real status pings, favorites and recently played.

use egui::{Id, Label, RichText, Sense, vec2};

use super::{Ctx, caption, confirm_modal, label, launch_and_report};
use crate::backend::BackendStatus;
use crate::model::server::{ServerEntry, ServerStore};
use crate::services::ping::ServerState;
use crate::ui::icons::paint_letter_tile;
use crate::ui::toasts::ToastKind;
use crate::ui::widgets::{
    card, column, cta_button, glass_panel, modal_frame, page_header, pill_tab, secondary_button,
    small_button, star_button, status_dot,
};
use crate::util::{now_secs, relative_time};

const AUTO_REFRESH_SECS: f64 = 60.0;

/// Which servers to list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Filter {
    #[default]
    All,
    Favorites,
    Recent,
}

struct Editor {
    entry: ServerEntry,
    is_new: bool,
    error: Option<String>,
}

/// Servers page state.
#[derive(Default)]
pub struct State {
    editor: Option<Editor>,
    remove: Option<String>,
    filter: Filter,
    last_ping_all: Option<f64>,
    /// Server to join when launching from the Play page.
    pub join_on_launch: Option<String>,
}

fn ping_one(c: &mut Ctx<'_>, entry: &ServerEntry) {
    match entry.validate() {
        Ok(addr) => c.pings.request(&entry.id, addr),
        Err(e) => {
            c.pings
                .states
                .insert(entry.id.clone(), ServerState::Offline(e.to_string()));
        }
    }
}

fn ping_all(c: &mut Ctx<'_>) {
    let servers = c.state.data.servers.servers.clone();
    for s in &servers {
        ping_one(c, s);
    }
    c.ui.servers.last_ping_all = Some(c.now);
}

fn visible(store: &ServerStore, filter: Filter) -> Vec<ServerEntry> {
    match filter {
        Filter::All => {
            let mut all = store.servers.clone();
            all.sort_by_key(|s| !s.favorite);
            all
        }
        Filter::Favorites => store
            .servers
            .iter()
            .filter(|s| s.favorite)
            .cloned()
            .collect(),
        Filter::Recent => store.recent(20).into_iter().cloned().collect(),
    }
}

/// Draw the Servers page.
pub fn show(ui: &mut egui::Ui, c: &mut Ctx<'_>) {
    page_header(
        ui,
        c.theme,
        "Servers",
        "Status comes from a real server list ping. SRV records are not looked up yet.",
    );
    if c.ui
        .servers
        .last_ping_all
        .is_none_or(|t| c.now - t > AUTO_REFRESH_SECS)
    {
        ping_all(c);
    }

    ui.horizontal(|ui| {
        if cta_button(ui, c.theme, "+  Add server").clicked() {
            c.ui.servers.editor = Some(Editor {
                entry: ServerEntry::new("", "", ""),
                is_new: true,
                error: None,
            });
        }
        let busy = c.pings.is_busy();
        ui.add_enabled_ui(!busy, |ui| {
            if secondary_button(ui, c.theme, if busy { "Refreshing…" } else { "Refresh" }).clicked()
            {
                ping_all(c);
            }
        });
        ui.add_space(12.0);
        for (f, name) in [
            (Filter::All, "All"),
            (Filter::Favorites, "Favorites"),
            (Filter::Recent, "Recently played"),
        ] {
            if pill_tab(ui, c.theme, name, c.ui.servers.filter == f) {
                c.ui.servers.filter = f;
            }
        }
    });
    ui.add_space(6.0);

    let list = visible(&c.state.data.servers, c.ui.servers.filter);
    if list.is_empty() {
        glass_panel(ui, c.theme, |ui| {
            ui.set_width(ui.available_width());
            let (title, body) = match c.ui.servers.filter {
                Filter::All => ("No servers yet", "Add a server to see its status here."),
                Filter::Favorites => ("No favorites", "Star a server to pin it here."),
                Filter::Recent => (
                    "Nothing played yet",
                    "Servers appear here after the game is launched into them.",
                ),
            };
            ui.label(RichText::new(title).size(17.0).strong().color(c.theme.text));
            caption(ui, c.theme, body);
        });
    }

    let ready = matches!(c.state.backend_status(), BackendStatus::Ready);
    let now = now_secs();
    let mut join = None;
    let mut edit = None;
    let mut remove = None;
    let mut favorite = None;

    egui::ScrollArea::vertical()
        .id_salt("servers_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for s in &list {
                let state = c.pings.states.get(&s.id).cloned();
                // Server icon from the ping, if any.
                let fav_key = format!("fav:{}", s.id);
                if let Some(ServerState::Online { status, .. }) = &state
                    && let Some(bytes) = &status.favicon_png
                {
                    c.textures.load_bytes(&fav_key, bytes, 64);
                }
                card(ui, c.theme, vec2(ui.available_width(), 104.0), false, |ui| {
                    ui.horizontal(|ui| {
                        let (r, _) = ui.allocate_exact_size(vec2(72.0, 72.0), Sense::hover());
                        match c.textures.get(&fav_key) {
                            Some(tex) => {
                                ui.painter().image(
                                    tex.id(),
                                    r,
                                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                                    egui::Color32::WHITE,
                                );
                            }
                            None => paint_letter_tile(ui.painter(), r, &s.name, &s.id),
                        }
                        let right_w = 250.0;
                        let mid_w = (ui.available_width() - right_w).max(120.0);
                        column(ui, vec2(mid_w, 72.0), |ui| {
                            ui.spacing_mut().item_spacing.y = 2.0;
                            ui.horizontal(|ui| {
                                ui.add(Label::new(RichText::new(&s.name).size(17.5).strong().color(c.theme.text)).truncate());
                                if star_button(ui, c.theme, s.favorite)
                                    .on_hover_text(if s.favorite { "Remove from favorites" } else { "Add to favorites" })
                                    .clicked()
                                {
                                    favorite = Some(s.id.clone());
                                }
                            });
                            let mut line = s.address.clone();
                            if !s.description.is_empty() {
                                line = format!("{line}  ·  {}", s.description);
                            }
                            ui.add(Label::new(RichText::new(line).size(13.0).color(c.theme.text_dim)).truncate());
                            match &state {
                                Some(ServerState::Online { status, .. }) if !status.motd.is_empty() => {
                                    let first = status.motd.lines().next().unwrap_or("");
                                    ui.add(Label::new(RichText::new(first).size(13.5).color(c.theme.text)).truncate());
                                }
                                _ => {
                                    if let Some(t) = s.last_played {
                                        caption(ui, c.theme, &format!("Played {}", relative_time(now, t)));
                                    }
                                }
                            }
                        });
                        column(ui, vec2(right_w - 8.0, 72.0), |ui| {
                            ui.spacing_mut().item_spacing.y = 3.0;
                            ui.horizontal(|ui| match &state {
                                Some(ServerState::Online { latency_ms, status }) => {
                                    status_dot(ui, c.theme.ok);
                                    ui.label(RichText::new("Online").strong().color(c.theme.ok));
                                    let players = match (status.players_online, status.players_max) {
                                        (Some(o), Some(m)) => format!("{o} / {m}"),
                                        (Some(o), None) => format!("{o}"),
                                        _ => "?".to_string(),
                                    };
                                    ui.label(RichText::new(format!("{players} players  ·  {latency_ms} ms")).size(13.0).color(c.theme.text_dim));
                                }
                                Some(ServerState::Pinging) | None => {
                                    status_dot(ui, c.theme.warn);
                                    ui.label(RichText::new("Pinging…").color(c.theme.warn));
                                }
                                Some(ServerState::Offline(why)) => {
                                    status_dot(ui, c.theme.danger);
                                    ui.label(RichText::new("Offline").strong().color(c.theme.danger))
                                        .on_hover_text(why.clone());
                                }
                            });
                            if let Some(ServerState::Online { status, .. }) = &state
                                && !status.version_name.is_empty()
                            {
                                ui.add(Label::new(RichText::new(&status.version_name).size(12.0).color(c.theme.text_faint)).truncate());
                            }
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 6.0;
                                let r = small_button(ui, c.theme, "Join", false);
                                if r.clicked() {
                                    join = Some(s.id.clone());
                                }
                                if !ready {
                                    r.on_hover_text("The Rust game client is not available yet, so joining is not possible.");
                                }
                                if small_button(ui, c.theme, "Edit", false).clicked() {
                                    edit = Some(s.id.clone());
                                }
                                if small_button(ui, c.theme, "Remove", true).clicked() {
                                    remove = Some(s.id.clone());
                                }
                            });
                        });
                    });
                });
                ui.add_space(2.0);
            }
        });

    if let Some(id) = favorite {
        c.state.data.servers.toggle_favorite(&id);
    }
    if let Some(id) = join {
        launch_and_report(c, Some(&id));
    }
    if let Some(id) = edit
        && let Some(entry) = c.state.data.servers.find(&id).cloned()
    {
        c.ui.servers.editor = Some(Editor {
            entry,
            is_new: false,
            error: None,
        });
    }
    if let Some(id) = remove {
        c.ui.servers.remove = Some(id);
    }
    editor_modal(c);
    remove_modal(c);
}

fn editor_modal(c: &mut Ctx<'_>) {
    let Some(editor) = c.ui.servers.editor.as_mut() else {
        return;
    };
    let theme = c.theme;
    let mut save = false;
    let mut cancel = false;
    let modal = egui::Modal::new(Id::new("server_editor"))
        .frame(modal_frame(theme))
        .show(c.egui, |ui| {
            ui.set_width(460.0);
            ui.label(
                RichText::new(if editor.is_new {
                    "Add server"
                } else {
                    "Edit server"
                })
                .size(22.0)
                .strong()
                .color(theme.text),
            );
            label(ui, theme, "Name");
            ui.add(egui::TextEdit::singleline(&mut editor.entry.name).desired_width(f32::INFINITY));
            label(ui, theme, "Address");
            ui.add(
                egui::TextEdit::singleline(&mut editor.entry.address)
                    .hint_text("play.example.org or play.example.org:25565")
                    .desired_width(f32::INFINITY),
            );
            label(ui, theme, "Description (your own note)");
            ui.add(
                egui::TextEdit::singleline(&mut editor.entry.description)
                    .desired_width(f32::INFINITY),
            );
            ui.checkbox(&mut editor.entry.favorite, "Favorite");
            if let Some(e) = &editor.error {
                ui.label(RichText::new(e).color(theme.danger));
            }
            ui.add_space(8.0);
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
        let mut entry = editor.entry.clone();
        entry.name = entry.name.trim().to_string();
        entry.address = entry.address.trim().to_string();
        entry.description = entry.description.trim().to_string();
        let is_new = editor.is_new;
        let result = if is_new {
            c.state.data.servers.add(entry.clone())
        } else {
            c.state
                .data
                .servers
                .update(entry.clone())
                .map(|()| entry.id.clone())
        };
        match result {
            Ok(_) => {
                c.ui.servers.editor = None;
                ping_one(c, &entry);
                c.toasts.push(
                    ToastKind::Success,
                    if is_new {
                        "Server added."
                    } else {
                        "Server saved."
                    },
                );
            }
            Err(e) => {
                if let Some(ed) = c.ui.servers.editor.as_mut() {
                    ed.error = Some(e.to_string());
                }
            }
        }
    } else if cancel {
        c.ui.servers.editor = None;
    }
}

fn remove_modal(c: &mut Ctx<'_>) {
    let Some(id) = c.ui.servers.remove.clone() else {
        return;
    };
    let name = c
        .state
        .data
        .servers
        .find(&id)
        .map(|s| s.name.clone())
        .unwrap_or_default();
    match confirm_modal(
        c.egui,
        c.theme,
        "remove_server",
        "Remove server?",
        &format!("\"{name}\" will be removed from your list."),
        "Remove",
    ) {
        Some(true) => {
            c.state.data.servers.remove(&id);
            c.pings.forget(&id);
            c.textures.evict(&format!("fav:{id}"));
            if c.ui.servers.join_on_launch.as_deref() == Some(id.as_str()) {
                c.ui.servers.join_on_launch = None;
            }
            c.ui.servers.remove = None;
            c.toasts.push(ToastKind::Info, "Server removed.");
        }
        Some(false) => c.ui.servers.remove = None,
        None => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visibility_filters() {
        let mut store = ServerStore::default();
        let a = store.add(ServerEntry::new("A", "a.example", "")).unwrap();
        let b = store.add(ServerEntry::new("B", "b.example", "")).unwrap();
        store.toggle_favorite(&b);
        store.mark_played(&a);
        assert_eq!(
            visible(&store, Filter::All)[0].id,
            b,
            "favorites sort first"
        );
        assert_eq!(visible(&store, Filter::Favorites).len(), 1);
        assert_eq!(visible(&store, Filter::Recent)[0].id, a);
    }
}
