//! The launcher pages.

pub mod accounts;
pub mod home;
pub mod installations;
pub mod play;
pub mod resource_packs;
pub mod screenshots;
pub mod servers;
pub mod settings;

use egui::{Context, Id, RichText, Ui};

use super::Page;
use super::theme::Theme;
use super::toasts::{ToastKind, Toasts};
use super::widgets::{danger_button, modal_frame, secondary_button};
use crate::backend::BackendStatus;
use crate::model::settings::AfterLaunch;
use crate::services::images::TextureCache;
use crate::services::ping::PingManager;
use crate::state::{LaunchBlocked, LauncherState};

/// Transient UI state of every page.
#[derive(Default)]
pub struct UiState {
    /// Requested navigation, applied by the app after the frame.
    pub next_page: Option<Page>,
    /// Home page state.
    pub home: home::State,
    /// Installations page state.
    pub installs: installations::State,
    /// Resource packs page state.
    pub packs: resource_packs::State,
    /// Servers page state.
    pub servers: servers::State,
    /// Accounts page state.
    pub accounts: accounts::State,
    /// Settings page state.
    pub settings: settings::State,
    /// Screenshots page state.
    pub shots: screenshots::State,
}

/// Everything a page needs for one frame.
pub struct Ctx<'a> {
    /// The egui context.
    pub egui: &'a Context,
    /// Launcher state.
    pub state: &'a mut LauncherState,
    /// Theme.
    pub theme: &'a Theme,
    /// Texture cache.
    pub textures: &'a mut TextureCache,
    /// Server pinger.
    pub pings: &'a mut PingManager,
    /// Notifications.
    pub toasts: &'a mut Toasts,
    /// Per-page UI state.
    pub ui: &'a mut UiState,
    /// Current time in seconds.
    pub now: f64,
}

/// Draw `page`.
pub fn show(page: Page, ui: &mut Ui, c: &mut Ctx<'_>) {
    match page {
        Page::Home => home::show(ui, c),
        Page::Play => play::show(ui, c),
        Page::Installations => installations::show(ui, c),
        Page::ResourcePacks => resource_packs::show(ui, c),
        Page::Servers => servers::show(ui, c),
        Page::Accounts => accounts::show(ui, c),
        Page::Settings => settings::show(ui, c),
        Page::Screenshots => screenshots::show(ui, c),
    }
}

/// Launch (optionally into a server) and report the outcome with a toast.
pub fn launch_and_report(c: &mut Ctx<'_>, server_id: Option<&str>) {
    match c.state.launch(server_id) {
        Ok(()) => {
            c.toasts.push(ToastKind::Success, "The game was started.");
            match c.state.data.settings.after_launch {
                AfterLaunch::KeepOpen => {}
                AfterLaunch::Minimize => c
                    .egui
                    .send_viewport_cmd(egui::ViewportCommand::Minimized(true)),
                AfterLaunch::Close => c.egui.send_viewport_cmd(egui::ViewportCommand::Close),
            }
        }
        Err(e) => {
            let kind = match e {
                LaunchBlocked::BackendUnavailable(_) => ToastKind::Warning,
                _ => ToastKind::Error,
            };
            c.toasts.push(kind, e.to_string());
        }
    }
}

/// Draw a chip describing whether the game backend is ready.
pub fn backend_chip(ui: &mut Ui, c: &Ctx<'_>) {
    match c.state.backend_status() {
        BackendStatus::Ready => {
            super::widgets::chip(ui, "Game client ready", c.theme.ok);
        }
        BackendStatus::Unavailable(reason) => {
            super::widgets::chip(ui, "Game client not available yet", c.theme.warn)
                .on_hover_text(reason);
        }
    }
}

/// A small dimmed caption.
pub fn caption(ui: &mut Ui, theme: &Theme, text: &str) {
    ui.label(RichText::new(text).size(13.0).color(theme.text_dim));
}

/// A field label.
pub fn label(ui: &mut Ui, theme: &Theme, text: &str) {
    ui.label(
        RichText::new(text)
            .size(13.5)
            .strong()
            .color(theme.text_dim),
    );
}

/// Show a confirmation dialog. Returns `Some(true)` when confirmed, `Some(false)` when
/// cancelled, `None` while it is still open.
pub fn confirm_modal(
    ctx: &Context,
    theme: &Theme,
    id: &str,
    title: &str,
    body: &str,
    confirm_label: &str,
) -> Option<bool> {
    let mut result = None;
    let modal = egui::Modal::new(Id::new(id))
        .frame(modal_frame(theme))
        .show(ctx, |ui| {
            ui.set_width(400.0);
            ui.label(RichText::new(title).size(20.0).strong().color(theme.text));
            ui.add_space(4.0);
            ui.label(RichText::new(body).size(14.5).color(theme.text_dim));
            ui.add_space(14.0);
            ui.horizontal(|ui| {
                if danger_button(ui, theme, confirm_label).clicked() {
                    result = Some(true);
                }
                if secondary_button(ui, theme, "Cancel").clicked() {
                    result = Some(false);
                }
            });
        });
    if modal.should_close() {
        result = result.or(Some(false));
    }
    result
}

/// Format a "last played" caption.
pub fn last_played_text(last: Option<u64>, now: u64) -> String {
    match last {
        Some(t) => format!("Last played {}", crate::util::relative_time(now, t)),
        None => "Never played".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn last_played_text_formats() {
        assert_eq!(last_played_text(None, 100), "Never played");
        assert_eq!(last_played_text(Some(0), 7200), "Last played 2 hours ago");
    }
}
