//! The eframe application: startup sequence, navigation and the per-frame UI.

use egui::{Context, Frame, LayerId, Margin, Rect, ViewportCommand, pos2, vec2};

use crate::model::settings::{Accent, UI_SCALE_RANGE};
use crate::services::images::TextureCache;
use crate::services::ping::PingManager;
use crate::startup::{Startup, StartupWork, StepState};
use crate::state::LauncherState;
use crate::ui::Page;
use crate::ui::pages::{self, Ctx, UiState};
use crate::ui::sidebar::{self, AccountSummary};
use crate::ui::splash;
use crate::ui::theme::Theme;
use crate::ui::toasts::{ToastKind, Toasts};
use crate::ui::widgets::{ease_out, glow, gradient_v};

const SAVE_INTERVAL_SECS: f64 = 0.5;

/// The real work performed at each startup stage.
struct Work<'a> {
    state: &'a mut LauncherState,
}

impl StartupWork for Work<'_> {
    fn init(&mut self) -> StepState {
        let before = self.state.data.warning_count();
        self.state.data.load_core();
        let d = &self.state.data;
        if d.warning_count() > before {
            return StepState::Failed(
                "A data file was unreadable; defaults were loaded".to_string(),
            );
        }
        StepState::Done(format!(
            "{} installation(s), {} server(s)",
            d.profiles.profiles.len(),
            d.servers.servers.len()
        ))
    }

    fn update_check(&mut self, enabled: bool) -> StepState {
        if enabled {
            StepState::Skipped(crate::updates::check_for_updates().message().to_string())
        } else {
            StepState::Skipped("Disabled in settings".to_string())
        }
    }

    fn load_accounts(&mut self) -> StepState {
        let before = self.state.data.warning_count();
        let n = self.state.data.load_accounts();
        if self.state.data.warning_count() > before {
            return StepState::Failed(
                "accounts.json was unreadable; no accounts loaded".to_string(),
            );
        }
        StepState::Done(format!("{n} account(s) loaded"))
    }
}

/// The launcher application.
pub struct LauncherApp {
    state: LauncherState,
    startup: Startup,
    theme: Theme,
    theme_key: (Accent, bool),
    zoom_applied: f32,
    textures: TextureCache,
    pings: PingManager,
    toasts: Toasts,
    ui: UiState,
    page: Page,
    page_anim: f32,
    last_time: f64,
    last_save: f64,
}

impl LauncherApp {
    /// Create the app. Settings must already be loaded into `state`.
    pub fn new(ctx: &Context, state: LauncherState) -> Self {
        let settings = &state.data.settings;
        let theme = Theme::new(settings.accent, settings.animations);
        theme.apply(ctx);
        ctx.set_zoom_factor(settings.ui_scale);
        let startup = Startup::new(settings.show_splash, settings.check_updates);
        Self {
            theme_key: (settings.accent, settings.animations),
            zoom_applied: settings.ui_scale,
            theme,
            startup,
            textures: TextureCache::new(ctx),
            pings: PingManager::new(Some(ctx.clone())),
            toasts: Toasts::default(),
            ui: UiState::default(),
            page: Page::Home,
            page_anim: 0.0,
            last_time: 0.0,
            last_save: 0.0,
            state,
        }
    }

    fn sync_theme(&mut self, ctx: &Context) {
        let s = &mut self.state.data.settings;
        let key = (s.accent, s.animations);
        if key != self.theme_key {
            self.theme = Theme::new(key.0, key.1);
            self.theme.apply(ctx);
            self.theme_key = key;
        }
        let live = ctx.zoom_factor();
        if (s.ui_scale - self.zoom_applied).abs() > 1e-3 {
            ctx.set_zoom_factor(s.ui_scale);
            self.zoom_applied = s.ui_scale;
        } else if (live - self.zoom_applied).abs() > 1e-3 {
            // The user zoomed with the keyboard: keep the setting in sync.
            s.ui_scale = live.clamp(*UI_SCALE_RANGE.start(), *UI_SCALE_RANGE.end());
            self.zoom_applied = s.ui_scale;
            if (live - s.ui_scale).abs() > 1e-3 {
                ctx.set_zoom_factor(s.ui_scale);
            }
        }
    }

    /// Write any unsaved changes to disk.
    fn save_now(&mut self) {
        let _ = self.state.data.save_if_changed();
    }

    fn set_page(&mut self, page: Page) {
        if page != self.page {
            self.page = page;
            self.page_anim = 0.0;
        }
    }

    /// Run one frame of the UI.
    pub fn ui(&mut self, ctx: &Context) {
        let now = ctx.input(|i| i.time);
        let dt = (now - self.last_time).clamp(0.0, 0.1) as f32;
        self.last_time = now;
        self.sync_theme(ctx);
        self.toasts.tick(now);

        if !self.startup.is_done() {
            self.startup.tick(
                dt,
                &mut Work {
                    state: &mut self.state,
                },
            );
            if !self.startup.is_done() {
                splash::show(ctx, &self.theme, &self.startup, now);
                return;
            }
            self.page_anim = 0.0;
        }
        self.running_ui(ctx, now, dt);
    }

    fn running_ui(&mut self, ctx: &Context, now: f64, dt: f32) {
        self.pings.poll();
        self.textures.poll();
        for warning in self.state.data.take_warnings() {
            self.toasts.push(ToastKind::Warning, warning);
        }

        if ctx.input(|i| i.key_pressed(egui::Key::F11)) {
            toggle_fullscreen(ctx);
        }

        paint_backdrop(ctx, &self.theme);

        let compact = sidebar::is_compact(
            self.state.data.settings.compact_sidebar,
            ctx.content_rect().width(),
        );
        let account = self
            .state
            .data
            .selected_account()
            .map(|a| (a.username().to_string(), a.status_label().to_string()));
        let summary = AccountSummary {
            name: account.as_ref().map(|(n, _)| n.as_str()),
            kind: account.as_ref().map(|(_, k)| k.as_str()).unwrap_or(""),
        };
        let out = sidebar::show(ctx, &self.theme, self.page, compact, &summary);
        if let Some(page) = out.page {
            self.set_page(page);
        }
        if out.toggle_fullscreen {
            toggle_fullscreen(ctx);
        }

        self.page_anim = if self.theme.animations {
            (self.page_anim + dt / 0.24).min(1.0)
        } else {
            1.0
        };
        if self.page_anim < 1.0 {
            ctx.request_repaint();
        }
        let t = ease_out(self.page_anim);
        let page = self.page;
        let theme = self.theme.clone();
        let frame = if page == Page::Home {
            Frame::new()
        } else {
            Frame::new().inner_margin(Margin::symmetric(32, 26))
        };
        egui::CentralPanel::default().frame(frame).show(ctx, |ui| {
            ui.set_opacity(t);
            if page != Page::Home {
                ui.add_space((1.0 - t) * 14.0);
            }
            let mut c = Ctx {
                egui: ctx,
                state: &mut self.state,
                theme: &theme,
                textures: &mut self.textures,
                pings: &mut self.pings,
                toasts: &mut self.toasts,
                ui: &mut self.ui,
                now,
            };
            pages::show(page, ui, &mut c);
        });

        if let Some(next) = self.ui.next_page.take() {
            self.set_page(next);
        }
        self.toasts.show(ctx, &theme);

        if now - self.last_save > SAVE_INTERVAL_SECS {
            for error in self.state.data.save_if_changed() {
                self.toasts.push(ToastKind::Error, error);
            }
            self.last_save = now;
        }
    }

    /// Press Play on the selected installation (used by tests).
    #[cfg(test)]
    fn press_play(&mut self, ctx: &Context) {
        let theme = self.theme.clone();
        let mut c = Ctx {
            egui: ctx,
            state: &mut self.state,
            theme: &theme,
            textures: &mut self.textures,
            pings: &mut self.pings,
            toasts: &mut self.toasts,
            ui: &mut self.ui,
            now: 0.0,
        };
        pages::launch_and_report(&mut c, None);
    }
}

fn toggle_fullscreen(ctx: &Context) {
    let current = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
    ctx.send_viewport_cmd(ViewportCommand::Fullscreen(!current));
}

fn paint_backdrop(ctx: &Context, theme: &Theme) {
    let painter = ctx.layer_painter(LayerId::background());
    let rect = ctx.content_rect();
    gradient_v(&painter, rect, theme.bg, egui::Color32::from_rgb(16, 9, 32));
    glow(
        &painter,
        Rect::from_center_size(
            pos2(rect.right() - 120.0, rect.top() + 60.0),
            vec2(8.0, 8.0),
        ),
        4,
        theme.accent_alpha(36),
        220,
    );
}

impl eframe::App for LauncherApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        self.ui(ctx);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.save_now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::NativeClientBackend;
    use crate::model::LauncherData;
    use crate::paths::LauncherDirs;
    use crate::test_support::{TempDir, encode_png};

    fn make_app(dir: &TempDir, splash: bool) -> (Context, LauncherApp) {
        let mut data = LauncherData::load_settings(LauncherDirs::from_root(dir.path()));
        data.settings.show_splash = splash;
        let state = LauncherState::new(data, Box::new(NativeClientBackend));
        let ctx = Context::default();
        let app = LauncherApp::new(&ctx, state);
        (ctx, app)
    }

    fn frame(ctx: &Context, app: &mut LauncherApp, time: f64, size: (f32, f32)) {
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(size.0, size.1))),
            time: Some(time),
            ..Default::default()
        };
        let _ = ctx.run(input, |ctx| app.ui(ctx));
    }

    fn seed_data(app: &mut LauncherApp, dir: &TempDir) {
        let d = &mut app.state.data;
        d.accounts.add_offline("Alex").unwrap();
        d.servers
            .add(crate::model::server::ServerEntry::new(
                "Local",
                "127.0.0.1:1",
                "test",
            ))
            .unwrap();
        let shots = dir.path().join("screenshots");
        std::fs::create_dir_all(&shots).unwrap();
        std::fs::write(shots.join("a.png"), encode_png(4, 4, &[200u8; 64])).unwrap();
        let packs = d.dirs.resource_packs_dir();
        std::fs::create_dir_all(&packs).unwrap();
        crate::model::resource_pack::tests::make_zip(
            &packs.join("pack.zip"),
            &[(
                "pack.mcmeta",
                crate::model::resource_pack::tests::MCMETA.as_bytes(),
            )],
        );
        d.packs.sync(&packs);
    }

    #[test]
    fn every_page_renders_at_several_sizes() {
        let dir = TempDir::new();
        let (ctx, mut app) = make_app(&dir, false);
        let mut t = 1.0;
        frame(&ctx, &mut app, t, (1280.0, 800.0));
        seed_data(&mut app, &dir);
        for size in [(1280.0, 800.0), (760.0, 520.0), (1920.0, 1080.0)] {
            for page in Page::ALL {
                app.set_page(page);
                for _ in 0..4 {
                    t += 0.1;
                    frame(&ctx, &mut app, t, size);
                }
                assert_eq!(app.page, page);
            }
        }
    }

    #[test]
    fn splash_runs_then_home_appears() {
        let dir = TempDir::new();
        let (ctx, mut app) = make_app(&dir, true);
        let mut t = 0.0;
        for _ in 0..200 {
            t += 0.05;
            frame(&ctx, &mut app, t, (1280.0, 800.0));
            if app.startup.is_done() {
                break;
            }
        }
        assert!(app.startup.is_done(), "startup must finish");
        assert!(
            matches!(app.startup.steps[1], StepState::Skipped(_)),
            "no update channel exists"
        );
        assert!(
            !app.state.data.profiles.profiles.is_empty(),
            "core data was loaded"
        );
    }

    #[test]
    fn startup_loads_existing_accounts() {
        let dir = TempDir::new();
        {
            let (ctx, mut app) = make_app(&dir, false);
            frame(&ctx, &mut app, 1.0, (1000.0, 700.0));
            app.state.data.accounts.add_offline("Steve").unwrap();
            app.save_now();
        }
        let (ctx, mut app) = make_app(&dir, false);
        frame(&ctx, &mut app, 1.0, (1000.0, 700.0));
        assert_eq!(app.state.data.accounts.accounts.len(), 1);
        assert_eq!(app.state.data.accounts.accounts[0].username(), "Steve");
    }

    #[test]
    fn unreadable_data_is_reported_not_hidden() {
        let dir = TempDir::new();
        std::fs::create_dir_all(dir.path().join("launcher")).unwrap();
        std::fs::write(dir.path().join("launcher").join("accounts.json"), "oops").unwrap();
        let (ctx, mut app) = make_app(&dir, false);
        frame(&ctx, &mut app, 1.0, (1000.0, 700.0));
        assert!(matches!(app.startup.steps[2], StepState::Failed(_)));
        assert!(app.state.data.accounts.accounts.is_empty());
        frame(&ctx, &mut app, 1.1, (1000.0, 700.0));
        assert!(
            app.toasts.len() >= 1,
            "the user is told about the unreadable file"
        );
    }

    #[test]
    fn pressing_play_without_backend_warns_and_launches_nothing() {
        let dir = TempDir::new();
        let (ctx, mut app) = make_app(&dir, false);
        frame(&ctx, &mut app, 1.0, (1000.0, 700.0));
        seed_data(&mut app, &dir);
        app.press_play(&ctx);
        assert_eq!(app.toasts.len(), 1);
        assert!(!app.state.last_launch.as_ref().unwrap().success);
        assert!(
            app.state
                .data
                .selected_profile()
                .unwrap()
                .last_played
                .is_none()
        );
    }

    #[test]
    fn settings_changes_are_persisted_on_exit() {
        let dir = TempDir::new();
        let (ctx, mut app) = make_app(&dir, false);
        frame(&ctx, &mut app, 1.0, (1000.0, 700.0));
        app.state.data.settings.accent = Accent::Orchid;
        app.state.data.settings.animations = false;
        frame(&ctx, &mut app, 2.0, (1000.0, 700.0));
        assert_eq!(app.theme_key, (Accent::Orchid, false));
        app.save_now();
        let reloaded = LauncherData::load_settings(LauncherDirs::from_root(dir.path()));
        assert_eq!(reloaded.settings.accent, Accent::Orchid);
        assert!(!reloaded.settings.animations);
    }

    #[test]
    fn navigation_request_changes_page() {
        let dir = TempDir::new();
        let (ctx, mut app) = make_app(&dir, false);
        frame(&ctx, &mut app, 1.0, (1000.0, 700.0));
        app.ui.next_page = Some(Page::Servers);
        frame(&ctx, &mut app, 1.1, (1000.0, 700.0));
        assert_eq!(app.page, Page::Servers);
    }
}
