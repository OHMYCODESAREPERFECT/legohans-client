//! Reusable glass-style widgets.

use egui::epaint::{Mesh, Shadow};
use egui::{
    Align, Align2, Color32, CornerRadius, CursorIcon, FontId, Frame, InnerResponse, Layout, Margin,
    Painter, Pos2, Rect, Response, RichText, Sense, Shape, Stroke, StrokeKind, Ui, UiBuilder, Vec2,
};

use super::theme::Theme;

/// Cubic ease-out from 0 to 1.
pub fn ease_out(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

/// Frame used for glass panels.
pub fn glass_frame(theme: &Theme) -> Frame {
    Frame::new()
        .fill(theme.glass)
        .stroke(Stroke::new(1.0, theme.border))
        .corner_radius(CornerRadius::same(16))
        .inner_margin(Margin::same(18))
        .shadow(Shadow {
            offset: [0, 10],
            blur: 28,
            spread: 0,
            color: Color32::from_black_alpha(110),
        })
}

/// Frame for dialogs: glass styling but nearly opaque so text stays legible over the page.
pub fn modal_frame(theme: &Theme) -> Frame {
    glass_frame(theme).fill(Color32::from_rgba_unmultiplied(22, 15, 38, 246))
}

/// A translucent glass panel with a subtle top highlight.
pub fn glass_panel<R>(
    ui: &mut Ui,
    theme: &Theme,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> InnerResponse<R> {
    let out = glass_frame(theme).show(ui, add_contents);
    let r = out.response.rect;
    ui.painter().line_segment(
        [
            Pos2::new(r.left() + 16.0, r.top() + 1.0),
            Pos2::new(r.right() - 16.0, r.top() + 1.0),
        ],
        Stroke::new(1.0, Color32::from_white_alpha(22)),
    );
    out
}

/// Soft colored glow around `rect`.
pub fn glow(painter: &Painter, rect: Rect, radius: u8, color: Color32, blur: u8) {
    painter.add(
        Shadow {
            offset: [0, 0],
            blur,
            spread: 2,
            color,
        }
        .as_shape(rect, CornerRadius::same(radius)),
    );
}

/// A rectangle with four corner colors.
pub fn gradient_rect(
    painter: &Painter,
    rect: Rect,
    tl: Color32,
    tr: Color32,
    bl: Color32,
    br: Color32,
) {
    let mut mesh = Mesh::default();
    mesh.colored_vertex(rect.left_top(), tl);
    mesh.colored_vertex(rect.right_top(), tr);
    mesh.colored_vertex(rect.left_bottom(), bl);
    mesh.colored_vertex(rect.right_bottom(), br);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(1, 3, 2);
    painter.add(Shape::mesh(mesh));
}

/// Vertical gradient from `top` to `bottom`.
pub fn gradient_v(painter: &Painter, rect: Rect, top: Color32, bottom: Color32) {
    gradient_rect(painter, rect, top, top, bottom, bottom);
}

/// Horizontal gradient from `left` to `right`.
pub fn gradient_h(painter: &Painter, rect: Rect, left: Color32, right: Color32) {
    gradient_rect(painter, rect, left, right, left, right);
}

fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    a.lerp_to_gamma(b, t.clamp(0.0, 1.0))
}

/// The big call-to-action button. When `enabled` is false it is drawn muted but still
/// reports clicks, so callers can explain why nothing happens.
pub fn primary_button(
    ui: &mut Ui,
    theme: &Theme,
    label: &str,
    size: Vec2,
    enabled: bool,
) -> Response {
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let hover = ui
        .ctx()
        .animate_bool_with_time(resp.id, resp.hovered(), theme.anim(0.18));
    let pulse = if theme.animations && enabled {
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(33));
        ((ui.input(|i| i.time) * 2.0).sin() as f32 * 0.5 + 0.5) * 0.35
    } else {
        0.0
    };
    let painter = ui.painter();
    let radius = (size.y / 2.0).min(32.0) as u8;
    let round = CornerRadius::same(radius);
    if enabled {
        let glow_alpha = (60.0 + 90.0 * hover + 60.0 * pulse) as u8;
        glow(painter, rect, radius, theme.accent_alpha(glow_alpha), 38);
        painter.rect_filled(
            rect,
            round,
            lerp_color(theme.accent, theme.accent_hi, 0.25 + 0.4 * hover),
        );
        // Glossy sheen over the top half.
        let sheen = Rect::from_min_max(rect.min, Pos2::new(rect.max.x, rect.center().y));
        painter.rect_filled(
            sheen,
            CornerRadius {
                nw: radius,
                ne: radius,
                sw: 0,
                se: 0,
            },
            Color32::from_white_alpha(34),
        );
    } else {
        painter.rect_filled(
            rect,
            round,
            Color32::from_rgba_unmultiplied(70, 58, 100, 160),
        );
    }
    painter.rect_stroke(
        rect,
        round,
        Stroke::new(
            1.5,
            if enabled {
                lerp_color(theme.accent_hi, Color32::WHITE, 0.4 + 0.4 * hover)
            } else {
                theme.border
            },
        ),
        StrokeKind::Inside,
    );
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        FontId::proportional(size.y * 0.38),
        if enabled {
            Color32::WHITE
        } else {
            theme.text_dim
        },
    );
    resp.on_hover_cursor(CursorIcon::PointingHand)
}

struct ButtonStyle {
    fg: Color32,
    accent: Color32,
    min_width: f32,
    font: f32,
    pad: f32,
    height: f32,
    filled: bool,
}

fn button_base(ui: &mut Ui, theme: &Theme, label: &str, s: ButtonStyle) -> Response {
    let ButtonStyle {
        fg,
        accent,
        min_width,
        font,
        pad,
        height,
        filled,
    } = s;
    let font = FontId::proportional(font);
    let galley = ui.painter().layout_no_wrap(label.to_string(), font, fg);
    let size = Vec2::new((galley.size().x + pad * 2.0).max(min_width), height);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let hover = ui
        .ctx()
        .animate_bool_with_time(resp.id, resp.hovered(), theme.anim(0.15));
    let enabled = ui.is_enabled();
    let painter = ui.painter();
    let base = if filled {
        theme.accent_alpha(170)
    } else {
        Color32::from_rgba_unmultiplied(44, 32, 72, 150)
    };
    let fill = lerp_color(
        base,
        accent,
        if filled {
            0.4 * hover
        } else {
            0.25 + 0.55 * hover
        },
    );
    let fill = if enabled {
        fill
    } else {
        fill.gamma_multiply(0.5)
    };
    painter.rect_filled(rect, CornerRadius::same(10), fill);
    painter.rect_stroke(
        rect,
        CornerRadius::same(10),
        Stroke::new(1.0, lerp_color(theme.border, accent, hover)),
        StrokeKind::Inside,
    );
    let text_pos = Pos2::new(
        rect.center().x - galley.size().x / 2.0,
        rect.center().y - galley.size().y / 2.0,
    );
    let color = if enabled { fg } else { fg.gamma_multiply(0.5) };
    painter.galley(text_pos, galley, color);
    if enabled {
        resp.on_hover_cursor(CursorIcon::PointingHand)
    } else {
        resp
    }
}

fn style(fg: Color32, accent: Color32) -> ButtonStyle {
    ButtonStyle {
        fg,
        accent,
        min_width: 0.0,
        font: 15.0,
        pad: 16.0,
        height: 34.0,
        filled: false,
    }
}

/// A standard secondary button.
pub fn secondary_button(ui: &mut Ui, theme: &Theme, label: &str) -> Response {
    button_base(ui, theme, label, style(theme.text, theme.accent))
}

/// A secondary button with a minimum width.
pub fn wide_button(ui: &mut Ui, theme: &Theme, label: &str, min_width: f32) -> Response {
    button_base(
        ui,
        theme,
        label,
        ButtonStyle {
            min_width,
            ..style(theme.text, theme.accent)
        },
    )
}

/// A destructive button.
pub fn danger_button(ui: &mut Ui, theme: &Theme, label: &str) -> Response {
    button_base(ui, theme, label, style(theme.danger, theme.danger))
}

/// A compact button for cards.
pub fn small_button(ui: &mut Ui, theme: &Theme, label: &str, danger: bool) -> Response {
    let (fg, accent) = if danger {
        (theme.danger, theme.danger)
    } else {
        (theme.text, theme.accent)
    };
    button_base(
        ui,
        theme,
        label,
        ButtonStyle {
            font: 13.5,
            pad: 11.0,
            height: 28.0,
            ..style(fg, accent)
        },
    )
}

/// A highlighted compact button (used for the "selected" state).
pub fn accent_button(ui: &mut Ui, theme: &Theme, label: &str) -> Response {
    button_base(
        ui,
        theme,
        label,
        ButtonStyle {
            font: 13.5,
            pad: 11.0,
            height: 28.0,
            filled: true,
            ..style(Color32::WHITE, theme.accent_hi)
        },
    )
}

/// A filled call-to-action button for page headers and dialogs.
pub fn cta_button(ui: &mut Ui, theme: &Theme, label: &str) -> Response {
    button_base(
        ui,
        theme,
        label,
        ButtonStyle {
            pad: 20.0,
            height: 38.0,
            filled: true,
            ..style(Color32::WHITE, theme.accent_hi)
        },
    )
}

/// A small rounded label.
pub fn chip(ui: &mut Ui, text: &str, color: Color32) -> Response {
    let font = FontId::proportional(12.5);
    let galley = ui.painter().layout_no_wrap(text.to_string(), font, color);
    let size = galley.size() + Vec2::new(18.0, 8.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::hover());
    let [r, g, b, _] = color.to_array();
    ui.painter().rect_filled(
        rect,
        CornerRadius::same(255),
        Color32::from_rgba_unmultiplied(r, g, b, 34),
    );
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(255),
        Stroke::new(1.0, Color32::from_rgba_unmultiplied(r, g, b, 90)),
        StrokeKind::Inside,
    );
    let pos = Pos2::new(rect.left() + 9.0, rect.center().y - galley.size().y / 2.0);
    ui.painter().galley(pos, galley, color);
    resp
}

/// An animated on/off switch.
pub fn toggle(ui: &mut Ui, theme: &Theme, on: &mut bool) -> Response {
    let size = Vec2::new(46.0, 26.0);
    let (rect, mut resp) = ui.allocate_exact_size(size, Sense::click());
    if resp.clicked() {
        *on = !*on;
        resp.mark_changed();
    }
    let t = ui
        .ctx()
        .animate_bool_with_time(resp.id, *on, theme.anim(0.16));
    let painter = ui.painter();
    let track = lerp_color(Color32::from_rgb(52, 42, 78), theme.accent, t);
    painter.rect_filled(rect, CornerRadius::same(13), track);
    painter.rect_stroke(
        rect,
        CornerRadius::same(13),
        Stroke::new(1.0, lerp_color(theme.border, theme.accent_hi, t)),
        StrokeKind::Inside,
    );
    let knob_x = rect.left() + 13.0 + t * (size.x - 26.0);
    if t > 0.01 {
        glow(
            painter,
            Rect::from_center_size(Pos2::new(knob_x, rect.center().y), Vec2::splat(12.0)),
            6,
            theme.accent_alpha((90.0 * t) as u8),
            14,
        );
    }
    painter.circle_filled(Pos2::new(knob_x, rect.center().y), 9.5, Color32::WHITE);
    resp.on_hover_cursor(CursorIcon::PointingHand)
}

/// Page title with an optional subtitle.
pub fn page_header(ui: &mut Ui, theme: &Theme, title: &str, subtitle: &str) {
    ui.label(RichText::new(title).size(30.0).strong().color(theme.text));
    if !subtitle.is_empty() {
        ui.label(RichText::new(subtitle).size(14.5).color(theme.text_dim));
    }
    ui.add_space(8.0);
}

/// Small section title.
pub fn section_title(ui: &mut Ui, theme: &Theme, text: &str) {
    ui.label(
        RichText::new(text.to_uppercase())
            .size(12.0)
            .strong()
            .color(theme.text_faint),
    );
}

/// A colored status dot.
pub fn status_dot(ui: &mut Ui, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(12.0), Sense::hover());
    glow(
        ui.painter(),
        Rect::from_center_size(rect.center(), Vec2::splat(4.0)),
        2,
        color.gamma_multiply(0.45),
        10,
    );
    ui.painter().circle_filled(rect.center(), 4.5, color);
}

fn star_points(center: Pos2, outer: f32) -> Vec<Pos2> {
    let inner = outer * 0.45;
    (0..10)
        .map(|i| {
            let angle = -std::f32::consts::FRAC_PI_2 + i as f32 * std::f32::consts::PI / 5.0;
            let r = if i % 2 == 0 { outer } else { inner };
            center + Vec2::new(angle.cos(), angle.sin()) * r
        })
        .collect()
}

/// A favorite star toggle button; returns the response.
pub fn star_button(ui: &mut Ui, theme: &Theme, filled: bool) -> Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(30.0), Sense::click());
    let hover = ui
        .ctx()
        .animate_bool_with_time(resp.id, resp.hovered(), theme.anim(0.12));
    let color = if filled {
        theme.warn
    } else {
        lerp_color(theme.text_faint, theme.warn, hover)
    };
    let pts = star_points(rect.center(), 10.0);
    ui.painter().add(Shape::convex_polygon(
        pts.clone(),
        if filled { color } else { Color32::TRANSPARENT },
        Stroke::new(1.6, color),
    ));
    resp.on_hover_cursor(CursorIcon::PointingHand)
}

/// A small up/down arrow button.
pub fn arrow_button(ui: &mut Ui, theme: &Theme, up: bool) -> Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(30.0, 24.0), Sense::click());
    let enabled = ui.is_enabled();
    let hover =
        ui.ctx()
            .animate_bool_with_time(resp.id, resp.hovered() && enabled, theme.anim(0.12));
    let color = if enabled {
        lerp_color(theme.text_dim, theme.accent_hi, hover)
    } else {
        theme.text_faint.gamma_multiply(0.4)
    };
    let c = rect.center();
    let d = if up { -1.0 } else { 1.0 };
    ui.painter().add(Shape::convex_polygon(
        vec![
            Pos2::new(c.x - 6.0, c.y - 3.0 * d),
            Pos2::new(c.x + 6.0, c.y - 3.0 * d),
            Pos2::new(c.x, c.y + 4.0 * d),
        ],
        color,
        Stroke::NONE,
    ));
    if enabled {
        resp.on_hover_cursor(CursorIcon::PointingHand)
    } else {
        resp
    }
}

/// A fixed-size vertical column. Use this instead of `Ui::allocate_ui` inside a horizontal
/// layout, which would otherwise lay the children out sideways too.
pub fn column<R>(ui: &mut Ui, size: Vec2, add_contents: impl FnOnce(&mut Ui) -> R) -> R {
    ui.allocate_ui_with_layout(size, Layout::top_down(Align::Min), |ui| {
        ui.set_min_size(size);
        add_contents(ui)
    })
    .inner
}

/// A clickable glass card of a fixed size. `add_contents` draws inside a padded child `Ui`.
pub fn card(
    ui: &mut Ui,
    theme: &Theme,
    size: Vec2,
    selected: bool,
    add_contents: impl FnOnce(&mut Ui),
) -> Response {
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let hover = ui
        .ctx()
        .animate_bool_with_time(resp.id, resp.hovered(), theme.anim(0.15));
    let sel = if selected { 1.0 } else { 0.0 };
    {
        let painter = ui.painter();
        let glow_alpha = (28.0 * hover + 46.0 * sel) as u8;
        if glow_alpha > 0 {
            glow(painter, rect, 16, theme.accent_alpha(glow_alpha), 26);
        }
        painter.rect_filled(
            rect,
            CornerRadius::same(16),
            lerp_color(
                theme.glass,
                Color32::from_rgba_unmultiplied(48, 34, 82, 215),
                hover.max(sel * 0.6),
            ),
        );
        painter.rect_stroke(
            rect,
            CornerRadius::same(16),
            Stroke::new(
                1.0 + 0.5 * sel,
                lerp_color(theme.border, theme.accent, hover.max(sel)),
            ),
            StrokeKind::Inside,
        );
    }
    let mut child = ui.new_child(
        UiBuilder::new()
            .max_rect(rect.shrink(16.0))
            .layout(Layout::top_down(Align::Min)),
    );
    add_contents(&mut child);
    resp
}

/// A tab-like pill used for filters. Returns true when clicked.
pub fn pill_tab(ui: &mut Ui, theme: &Theme, label: &str, selected: bool) -> bool {
    let font = FontId::proportional(14.5);
    let color = if selected {
        Color32::WHITE
    } else {
        theme.text_dim
    };
    let galley = ui.painter().layout_no_wrap(label.to_string(), font, color);
    let size = Vec2::new(galley.size().x + 28.0, 32.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let t = ui
        .ctx()
        .animate_bool_with_time(resp.id, selected, theme.anim(0.15));
    let hover =
        ui.ctx()
            .animate_bool_with_time(resp.id.with("h"), resp.hovered(), theme.anim(0.12));
    let painter = ui.painter();
    painter.rect_filled(
        rect,
        CornerRadius::same(255),
        lerp_color(
            Color32::from_rgba_unmultiplied(60, 44, 96, (40.0 + 50.0 * hover) as u8),
            theme.accent_alpha(150),
            t,
        ),
    );
    if t > 0.01 {
        glow(painter, rect, 16, theme.accent_alpha((60.0 * t) as u8), 16);
    }
    let pos = Pos2::new(
        rect.center().x - galley.size().x / 2.0,
        rect.center().y - galley.size().y / 2.0,
    );
    painter.galley(pos, galley, color);
    resp.on_hover_cursor(CursorIcon::PointingHand).clicked()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn easing_is_bounded_and_monotonic() {
        assert_eq!(ease_out(-1.0), 0.0);
        assert_eq!(ease_out(2.0), 1.0);
        let mut last = 0.0;
        for i in 0..=10 {
            let v = ease_out(i as f32 / 10.0);
            assert!(v >= last);
            last = v;
        }
    }

    #[test]
    fn star_has_ten_points() {
        assert_eq!(star_points(Pos2::ZERO, 10.0).len(), 10);
    }
}
