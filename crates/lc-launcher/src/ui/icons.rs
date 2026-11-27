//! Vector icons drawn with painter primitives (no font glyphs needed).

use egui::epaint::Hsva;
use egui::{
    Align2, Color32, CornerRadius, FontId, Painter, Pos2, Rect, Shape, Stroke, StrokeKind, Vec2,
};

use super::Page;
use super::widgets::{glow, gradient_v};
use crate::model::profile::ProfileIcon;
use crate::util::hash_str;

fn pts(center: Pos2, u: f32, list: &[(f32, f32)]) -> Vec<Pos2> {
    list.iter()
        .map(|(x, y)| Pos2::new(center.x + x * u, center.y + y * u))
        .collect()
}

/// Paint the sidebar icon of `page` centered at `center`, `size` pixels wide.
pub fn paint_nav_icon(painter: &Painter, center: Pos2, size: f32, page: Page, color: Color32) {
    let u = size / 2.0;
    let stroke = Stroke::new((size / 12.0).max(1.4), color);
    match page {
        Page::Home => {
            painter.add(Shape::line(
                pts(center, u, &[(-1.0, -0.05), (0.0, -0.9), (1.0, -0.05)]),
                stroke,
            ));
            painter.add(Shape::line(
                pts(
                    center,
                    u,
                    &[(-0.7, -0.15), (-0.7, 0.85), (0.7, 0.85), (0.7, -0.15)],
                ),
                stroke,
            ));
            painter.add(Shape::line(
                pts(
                    center,
                    u,
                    &[(-0.22, 0.85), (-0.22, 0.3), (0.22, 0.3), (0.22, 0.85)],
                ),
                stroke,
            ));
        }
        Page::Play => {
            painter.add(Shape::convex_polygon(
                pts(center, u, &[(-0.5, -0.85), (0.9, 0.0), (-0.5, 0.85)]),
                color,
                Stroke::new(1.0, color),
            ));
        }
        Page::Installations => {
            painter.add(Shape::closed_line(
                pts(
                    center,
                    u,
                    &[
                        (0.0, -1.0),
                        (0.87, -0.5),
                        (0.87, 0.5),
                        (0.0, 1.0),
                        (-0.87, 0.5),
                        (-0.87, -0.5),
                    ],
                ),
                stroke,
            ));
            painter.add(Shape::line(
                pts(center, u, &[(-0.87, -0.5), (0.0, 0.0), (0.87, -0.5)]),
                stroke,
            ));
            painter.line_segment([center, Pos2::new(center.x, center.y + u)], stroke);
        }
        Page::ResourcePacks => {
            painter.add(Shape::closed_line(
                pts(
                    center,
                    u,
                    &[(0.0, -0.95), (1.0, -0.45), (0.0, 0.05), (-1.0, -0.45)],
                ),
                stroke,
            ));
            painter.add(Shape::line(
                pts(center, u, &[(-1.0, 0.05), (0.0, 0.55), (1.0, 0.05)]),
                stroke,
            ));
            painter.add(Shape::line(
                pts(center, u, &[(-1.0, 0.55), (0.0, 1.0), (1.0, 0.55)]),
                stroke,
            ));
        }
        Page::Servers => {
            for (y0, y1) in [(-0.9, -0.1), (0.1, 0.9)] {
                painter.rect_stroke(
                    Rect::from_min_max(
                        Pos2::new(center.x - 0.95 * u, center.y + y0 * u),
                        Pos2::new(center.x + 0.95 * u, center.y + y1 * u),
                    ),
                    CornerRadius::same((u * 0.2) as u8),
                    stroke,
                    StrokeKind::Inside,
                );
                painter.circle_filled(
                    Pos2::new(center.x - 0.55 * u, center.y + (y0 + y1) / 2.0 * u),
                    u * 0.1,
                    color,
                );
            }
        }
        Page::Accounts => {
            painter.circle_stroke(Pos2::new(center.x, center.y - 0.4 * u), 0.42 * u, stroke);
            let arc: Vec<Pos2> = (0..=16)
                .map(|i| {
                    let a = std::f32::consts::PI + i as f32 / 16.0 * std::f32::consts::PI;
                    Pos2::new(
                        center.x + 0.9 * u * a.cos(),
                        center.y + 1.0 * u + 0.75 * u * a.sin(),
                    )
                })
                .collect();
            painter.add(Shape::line(arc, stroke));
        }
        Page::SkinEditor => {
            painter.circle_stroke(Pos2::new(center.x, center.y - 0.45 * u), 0.38 * u, stroke);
            painter.add(Shape::line(
                pts(
                    center,
                    u,
                    &[
                        (-0.75, 1.0),
                        (-0.65, 0.25),
                        (-0.2, 0.0),
                        (0.2, 0.0),
                        (0.65, 0.25),
                        (0.75, 1.0),
                    ],
                ),
                stroke,
            ));
            painter.line_segment(
                [
                    Pos2::new(center.x + 0.5 * u, center.y + 0.5 * u),
                    Pos2::new(center.x + 1.0 * u, center.y),
                ],
                stroke,
            );
        }
        Page::Settings => {
            painter.circle_stroke(center, 0.3 * u, stroke);
            painter.circle_stroke(center, 0.62 * u, stroke);
            for i in 0..8 {
                let a = i as f32 * std::f32::consts::FRAC_PI_4;
                let dir = Vec2::new(a.cos(), a.sin());
                painter.line_segment(
                    [center + dir * (0.62 * u), center + dir * (0.98 * u)],
                    stroke,
                );
            }
        }
        Page::Screenshots => {
            painter.rect_stroke(
                Rect::from_min_max(
                    Pos2::new(center.x - u, center.y - 0.75 * u),
                    Pos2::new(center.x + u, center.y + 0.75 * u),
                ),
                CornerRadius::same((u * 0.2) as u8),
                stroke,
                StrokeKind::Inside,
            );
            painter.circle_stroke(
                Pos2::new(center.x + 0.45 * u, center.y - 0.25 * u),
                0.17 * u,
                stroke,
            );
            painter.add(Shape::line(
                pts(
                    center,
                    u,
                    &[
                        (-0.85, 0.6),
                        (-0.25, -0.1),
                        (0.15, 0.4),
                        (0.4, 0.15),
                        (0.85, 0.6),
                    ],
                ),
                stroke,
            ));
        }
    }
}

fn block_colors(icon: ProfileIcon) -> (Color32, Color32, Color32) {
    let c = Color32::from_rgb;
    match icon {
        ProfileIcon::Grass => (c(104, 178, 64), c(128, 88, 54), c(104, 70, 42)),
        ProfileIcon::Stone => (c(156, 156, 166), c(120, 120, 130), c(96, 96, 106)),
        ProfileIcon::Obsidian => (c(58, 34, 92), c(34, 20, 58), c(22, 12, 40)),
        ProfileIcon::Amethyst => (c(186, 130, 240), c(138, 90, 196), c(108, 68, 164)),
        ProfileIcon::Diamond => (c(120, 236, 236), c(74, 184, 196), c(54, 150, 164)),
        ProfileIcon::Gold => (c(252, 218, 88), c(220, 174, 54), c(184, 138, 40)),
    }
}

/// Paint an isometric block of the given style centered in `rect`.
pub fn paint_block_icon(painter: &Painter, rect: Rect, icon: ProfileIcon, glow_color: Color32) {
    let c = rect.center();
    let r = rect.width().min(rect.height()) * 0.5;
    let (top, left, right) = block_colors(icon);
    let p = |x: f32, y: f32| Pos2::new(c.x + x * r, c.y + y * r);
    glow(
        painter,
        Rect::from_center_size(c, Vec2::splat(r)),
        (r * 0.3) as u8,
        glow_color,
        (r * 0.9).min(48.0) as u8,
    );
    let edge = Stroke::new(1.0, Color32::from_black_alpha(90));
    let (t, ur, lr, b, ll, ul, m) = (
        p(0.0, -1.0),
        p(0.866, -0.5),
        p(0.866, 0.5),
        p(0.0, 1.0),
        p(-0.866, 0.5),
        p(-0.866, -0.5),
        p(0.0, 0.0),
    );
    painter.add(Shape::convex_polygon(vec![ul, m, b, ll], left, edge));
    painter.add(Shape::convex_polygon(vec![m, ur, lr, b], right, edge));
    painter.add(Shape::convex_polygon(vec![t, ur, m, ul], top, edge));
    // A few texture pixels on the top face.
    for (i, (x, y)) in [(-0.3, -0.45), (0.25, -0.55), (0.1, -0.2), (-0.05, -0.7)]
        .iter()
        .enumerate()
    {
        let shade = if i % 2 == 0 { 28 } else { 18 };
        painter.rect_filled(
            Rect::from_center_size(p(*x, *y), Vec2::splat(r * 0.14)),
            CornerRadius::ZERO,
            Color32::from_black_alpha(shade),
        );
    }
}

/// A colored rounded tile with an initial letter, used when there is no icon image.
pub fn paint_letter_tile(painter: &Painter, rect: Rect, text: &str, seed: &str) {
    let h = hash_str(seed);
    let hue = 0.68 + (h % 1000) as f32 / 1000.0 * 0.28;
    let base: Color32 = Hsva::new(hue.fract(), 0.55, 0.62, 1.0).into();
    let dark: Color32 = Hsva::new(hue.fract(), 0.65, 0.34, 1.0).into();
    let radius = (rect.width() * 0.22) as u8;
    painter.rect_filled(rect, CornerRadius::same(radius), dark);
    let inner = Rect::from_min_max(
        rect.min,
        Pos2::new(rect.max.x, rect.center().y + rect.height() * 0.1),
    );
    painter.rect_filled(
        inner,
        CornerRadius {
            nw: radius,
            ne: radius,
            sw: 0,
            se: 0,
        },
        base,
    );
    gradient_v(
        painter,
        Rect::from_min_max(
            Pos2::new(rect.left() + 1.0, rect.center().y + rect.height() * 0.1),
            Pos2::new(rect.right() - 1.0, rect.bottom() - radius as f32 * 0.5),
        ),
        base,
        dark,
    );
    painter.rect_stroke(
        rect,
        CornerRadius::same(radius),
        Stroke::new(1.0, Color32::from_white_alpha(40)),
        StrokeKind::Inside,
    );
    let letter: String = text
        .chars()
        .find(|c| c.is_alphanumeric())
        .map(|c| c.to_uppercase().collect())
        .unwrap_or_else(|| "?".to_string());
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        letter,
        FontId::proportional(rect.height() * 0.5),
        Color32::WHITE,
    );
}

/// Paint a small spinner arc.
pub fn paint_spinner(painter: &Painter, center: Pos2, radius: f32, time: f64, color: Color32) {
    let start = (time * 4.0) as f32;
    let arc: Vec<Pos2> = (0..=18)
        .map(|i| {
            let a = start + i as f32 / 18.0 * 4.2;
            center + Vec2::new(a.cos(), a.sin()) * radius
        })
        .collect();
    painter.add(Shape::line(arc, Stroke::new(2.0, color)));
}

/// Paint a check mark.
pub fn paint_check(painter: &Painter, center: Pos2, size: f32, color: Color32) {
    let u = size / 2.0;
    painter.add(Shape::line(
        pts(center, u, &[(-0.7, 0.05), (-0.2, 0.6), (0.8, -0.6)]),
        Stroke::new(2.2, color),
    ));
}

/// Paint a cross.
pub fn paint_cross(painter: &Painter, center: Pos2, size: f32, color: Color32) {
    let u = size / 2.0 * 0.7;
    let s = Stroke::new(2.2, color);
    painter.line_segment([center + Vec2::new(-u, -u), center + Vec2::new(u, u)], s);
    painter.line_segment([center + Vec2::new(-u, u), center + Vec2::new(u, -u)], s);
}

/// Paint a dash.
pub fn paint_dash(painter: &Painter, center: Pos2, size: f32, color: Color32) {
    let u = size / 2.0 * 0.7;
    painter.line_segment(
        [center + Vec2::new(-u, 0.0), center + Vec2::new(u, 0.0)],
        Stroke::new(2.2, color),
    );
}
