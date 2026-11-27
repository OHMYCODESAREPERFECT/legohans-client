//! Splash screen shown while the launcher starts.

use egui::{Align2, Color32, Context, CornerRadius, FontId, Frame, Pos2, Rect, Sense, Vec2};

use super::icons::{paint_block_icon, paint_check, paint_cross, paint_dash, paint_spinner};
use super::theme::Theme;
use super::widgets::{glow, gradient_v};
use crate::model::profile::ProfileIcon;
use crate::startup::{Stage, Startup, StepState};

const STEP_NAMES: [&str; 3] = [
    "Preparing launcher",
    "Checking for updates",
    "Loading accounts",
];

fn stage_index(stage: Stage) -> usize {
    match stage {
        Stage::Splash => 0,
        Stage::UpdateCheck => 1,
        Stage::Accounts => 2,
        Stage::Done => 3,
    }
}

/// Draw the splash screen for the current startup state.
pub fn show(ctx: &Context, theme: &Theme, startup: &Startup, now: f64) {
    egui::CentralPanel::default()
        .frame(Frame::new().fill(theme.bg))
        .show(ctx, |ui| {
            let rect = ui.max_rect();
            let painter = ui.painter();
            gradient_v(
                painter,
                rect,
                Color32::from_rgb(8, 5, 16),
                Color32::from_rgb(22, 10, 44),
            );
            let center = Pos2::new(rect.center().x, rect.center().y - 70.0);

            // Pulsing logo.
            let pulse = if theme.animations {
                ((now * 2.2).sin() as f32 * 0.5 + 0.5) * 0.5
            } else {
                0.25
            };
            let logo = Rect::from_center_size(center, Vec2::splat(120.0));
            glow(
                painter,
                logo,
                40,
                theme.accent_alpha((50.0 + 70.0 * pulse) as u8),
                90,
            );
            paint_block_icon(painter, logo, ProfileIcon::Amethyst, theme.accent_alpha(80));

            painter.text(
                Pos2::new(center.x, center.y + 98.0),
                Align2::CENTER_CENTER,
                "LEGOHAN'S CLIENT",
                FontId::proportional(32.0),
                theme.text,
            );
            painter.text(
                Pos2::new(center.x, center.y + 128.0),
                Align2::CENTER_CENTER,
                "Minecraft Java Edition 1.21.11  ·  written in Rust",
                FontId::proportional(14.5),
                theme.text_dim,
            );

            // Progress bar.
            let bar_w = 340.0f32.min(rect.width() - 80.0);
            let bar = Rect::from_center_size(
                Pos2::new(center.x, center.y + 168.0),
                Vec2::new(bar_w, 6.0),
            );
            painter.rect_filled(bar, CornerRadius::same(3), Color32::from_white_alpha(18));
            let fill = Rect::from_min_size(bar.min, Vec2::new(bar_w * startup.progress(), 6.0));
            glow(painter, fill, 3, theme.accent_alpha(110), 14);
            painter.rect_filled(fill, CornerRadius::same(3), theme.accent_hi);

            // Steps.
            let current = stage_index(startup.stage());
            let mut y = center.y + 204.0;
            for (i, name) in STEP_NAMES.iter().enumerate() {
                let row_center = Pos2::new(center.x - bar_w / 2.0 + 10.0, y);
                let (color, message) = if i > current {
                    (theme.text_faint, "waiting".to_string())
                } else {
                    match &startup.steps[i] {
                        StepState::Pending => (theme.text_faint, String::new()),
                        StepState::Done(m) => (theme.ok, m.clone()),
                        StepState::Skipped(m) => (theme.warn, m.clone()),
                        StepState::Failed(m) => (theme.danger, m.clone()),
                    }
                };
                if i == current {
                    paint_spinner(painter, row_center, 6.0, now, theme.accent_hi);
                } else if i > current {
                    paint_dash(painter, row_center, 12.0, theme.text_faint);
                } else {
                    match &startup.steps[i] {
                        StepState::Done(_) => paint_check(painter, row_center, 14.0, theme.ok),
                        StepState::Skipped(_) => paint_dash(painter, row_center, 12.0, theme.warn),
                        StepState::Failed(_) => {
                            paint_cross(painter, row_center, 12.0, theme.danger)
                        }
                        StepState::Pending => {
                            paint_dash(painter, row_center, 12.0, theme.text_faint)
                        }
                    }
                }
                painter.text(
                    Pos2::new(row_center.x + 20.0, y),
                    Align2::LEFT_CENTER,
                    *name,
                    FontId::proportional(14.5),
                    if i <= current {
                        theme.text
                    } else {
                        theme.text_faint
                    },
                );
                painter.text(
                    Pos2::new(center.x + bar_w / 2.0, y),
                    Align2::RIGHT_CENTER,
                    truncate(&message, 46),
                    FontId::proportional(12.5),
                    color,
                );
                y += 28.0;
            }
            let _ = ui.allocate_rect(rect, Sense::hover());
        });
    ctx.request_repaint_after(std::time::Duration::from_millis(16));
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let cut: String = s.chars().take(max - 1).collect();
        format!("{cut}…")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncation_adds_ellipsis() {
        assert_eq!(truncate("short", 10), "short");
        assert_eq!(truncate("abcdefghij", 5), "abcd…");
    }

    #[test]
    fn stage_indices_increase() {
        assert!(stage_index(Stage::Splash) < stage_index(Stage::UpdateCheck));
        assert!(stage_index(Stage::Accounts) < stage_index(Stage::Done));
    }
}
