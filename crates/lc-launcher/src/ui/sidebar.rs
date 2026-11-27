//! The left navigation sidebar.

use egui::{
    Align2, Color32, Context, CornerRadius, CursorIcon, FontId, Frame, Id, Margin, Pos2, Rect,
    RichText, Sense, Stroke, StrokeKind, Vec2,
};

use super::Page;
use super::icons::{paint_block_icon, paint_letter_tile, paint_nav_icon};
use super::theme::Theme;
use super::widgets::glow;
use crate::model::profile::ProfileIcon;

const EXPANDED: f32 = 232.0;
const COMPACT: f32 = 72.0;
/// Window width below which the sidebar collapses on its own.
pub const AUTO_COMPACT_BELOW: f32 = 920.0;

/// What the user did in the sidebar this frame.
#[derive(Debug, Default)]
pub struct SidebarOutput {
    /// A page was clicked.
    pub page: Option<Page>,
    /// The fullscreen button was clicked.
    pub toggle_fullscreen: bool,
}

/// The account summary shown at the bottom.
pub struct AccountSummary<'a> {
    /// Display name, or `None` if there are no accounts.
    pub name: Option<&'a str>,
    /// Short type label.
    pub kind: &'a str,
}

/// Whether the sidebar should be compact for this window width.
pub fn is_compact(setting: bool, window_width: f32) -> bool {
    setting || window_width < AUTO_COMPACT_BELOW
}

/// Draw the sidebar.
pub fn show(
    ctx: &Context,
    theme: &Theme,
    current: Page,
    compact: bool,
    account: &AccountSummary<'_>,
) -> SidebarOutput {
    let mut out = SidebarOutput::default();
    let width = ctx.animate_value_with_time(
        Id::new("sidebar_width"),
        if compact { COMPACT } else { EXPANDED },
        theme.anim(0.2),
    );
    let expanded = width > 150.0;
    egui::SidePanel::left("sidebar")
        .exact_width(width)
        .resizable(false)
        .frame(
            Frame::new()
                .fill(theme.bg_alt)
                .stroke(Stroke::new(1.0, theme.border))
                .inner_margin(Margin::symmetric(12, 18)),
        )
        .show(ctx, |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            // Brand.
            let brand_h = 46.0;
            let (rect, _) =
                ui.allocate_exact_size(Vec2::new(ui.available_width(), brand_h), Sense::hover());
            let icon_rect = Rect::from_center_size(
                Pos2::new(rect.left() + 20.0, rect.center().y),
                Vec2::splat(34.0),
            );
            paint_block_icon(
                ui.painter(),
                icon_rect,
                ProfileIcon::Amethyst,
                theme.accent_alpha(70),
            );
            if expanded {
                let alpha = ((width - 150.0) / 80.0).clamp(0.0, 1.0);
                let fade = |c: Color32| c.gamma_multiply(alpha);
                ui.painter().text(
                    Pos2::new(rect.left() + 46.0, rect.center().y - 8.0),
                    Align2::LEFT_CENTER,
                    "LEGOHAN'S",
                    FontId::proportional(16.0),
                    fade(theme.text),
                );
                ui.painter().text(
                    Pos2::new(rect.left() + 46.0, rect.center().y + 9.0),
                    Align2::LEFT_CENTER,
                    "CLIENT  ·  RUST",
                    FontId::proportional(11.0),
                    fade(theme.accent_hi),
                );
            }
            ui.add_space(14.0);

            for page in Page::ALL {
                if nav_item(ui, theme, page, page == current, expanded, width) {
                    out.page = Some(page);
                }
            }

            ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                ui.spacing_mut().item_spacing.y = 8.0;
                let label = if expanded { "Fullscreen  (F11)" } else { "F11" };
                if super::widgets::secondary_button(ui, theme, label)
                    .on_hover_text("Toggle fullscreen")
                    .clicked()
                {
                    out.toggle_fullscreen = true;
                }
                account_chip(ui, theme, account, expanded);
            });
        });
    out
}

fn account_chip(ui: &mut egui::Ui, theme: &Theme, account: &AccountSummary<'_>, expanded: bool) {
    let size = Vec2::new(ui.available_width(), 50.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::hover());
    ui.painter().rect_filled(
        rect,
        CornerRadius::same(12),
        Color32::from_rgba_unmultiplied(60, 44, 96, 70),
    );
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(12),
        Stroke::new(1.0, theme.border),
        StrokeKind::Inside,
    );
    let tile = Rect::from_center_size(
        Pos2::new(rect.left() + 26.0, rect.center().y),
        Vec2::splat(32.0),
    );
    match account.name {
        Some(name) => {
            paint_letter_tile(ui.painter(), tile, name, name);
            if expanded {
                ui.painter().text(
                    Pos2::new(rect.left() + 50.0, rect.center().y - 8.0),
                    Align2::LEFT_CENTER,
                    name,
                    FontId::proportional(14.0),
                    theme.text,
                );
                ui.painter().text(
                    Pos2::new(rect.left() + 50.0, rect.center().y + 9.0),
                    Align2::LEFT_CENTER,
                    account.kind,
                    FontId::proportional(11.5),
                    theme.text_faint,
                );
            }
            resp.on_hover_text(format!("{name} ({})", account.kind));
        }
        None => {
            paint_letter_tile(ui.painter(), tile, "?", "none");
            if expanded {
                ui.painter().text(
                    Pos2::new(rect.left() + 50.0, rect.center().y),
                    Align2::LEFT_CENTER,
                    "No account yet",
                    FontId::proportional(13.5),
                    theme.text_dim,
                );
            }
        }
    }
}

fn nav_item(
    ui: &mut egui::Ui,
    theme: &Theme,
    page: Page,
    active: bool,
    expanded: bool,
    width: f32,
) -> bool {
    let size = Vec2::new(ui.available_width(), 44.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let hover = ui
        .ctx()
        .animate_bool_with_time(resp.id, resp.hovered(), theme.anim(0.14));
    let on = ui
        .ctx()
        .animate_bool_with_time(resp.id.with("on"), active, theme.anim(0.2));
    let painter = ui.painter();
    if on > 0.01 || hover > 0.01 {
        let fill_alpha = (28.0 * hover + 52.0 * on) as u8;
        painter.rect_filled(rect, CornerRadius::same(12), theme.accent_alpha(fill_alpha));
    }
    if on > 0.01 {
        let bar = Rect::from_center_size(
            Pos2::new(rect.left() + 1.5, rect.center().y),
            Vec2::new(3.0, 22.0 * on),
        );
        glow(painter, bar, 2, theme.accent_alpha((140.0 * on) as u8), 14);
        painter.rect_filled(bar, CornerRadius::same(2), theme.accent_hi);
    }
    let color = if active {
        Color32::WHITE
    } else {
        Color32::from_rgb(172, 162, 198).lerp_to_gamma(Color32::WHITE, hover * 0.7)
    };
    let icon_center = Pos2::new(rect.left() + 26.0, rect.center().y);
    paint_nav_icon(
        painter,
        icon_center,
        20.0,
        page,
        if active { theme.accent_hi } else { color },
    );
    if expanded {
        let alpha = ((width - 150.0) / 80.0).clamp(0.0, 1.0);
        painter.text(
            Pos2::new(rect.left() + 52.0, rect.center().y),
            Align2::LEFT_CENTER,
            page.title(),
            FontId::proportional(15.5),
            color.gamma_multiply(alpha),
        );
    }
    let resp = resp.on_hover_cursor(CursorIcon::PointingHand);
    if !expanded {
        resp.clone().on_hover_text(RichText::new(page.title()));
    }
    resp.clicked()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_when_setting_or_narrow() {
        assert!(is_compact(true, 2000.0));
        assert!(is_compact(false, 800.0));
        assert!(!is_compact(false, 1280.0));
    }
}
