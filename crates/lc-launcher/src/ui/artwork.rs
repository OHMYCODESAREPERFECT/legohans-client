//! Home page artwork: a generated voxel landscape, or the user's own vanilla panorama.
//!
//! The generated landscape is original artwork drawn in code. Mojang's artwork is never
//! bundled; the panorama is only used if it can be read from the user's own install.

use egui::{Color32, CornerRadius, Painter, Pos2, Rect, TextureHandle, Vec2, pos2};

use super::widgets::{glow, gradient_v};
use crate::util::hash_str;

fn hash01(seed: u64, n: i64) -> f32 {
    let mut x =
        (n as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15) ^ seed.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x ^= x >> 31;
    x = x.wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^= x >> 29;
    (x & 0xffff) as f32 / 65535.0
}

/// Smooth value noise in 0..1 along one axis.
fn noise(seed: u64, x: f32) -> f32 {
    let i = x.floor();
    let f = x - i;
    let s = f * f * (3.0 - 2.0 * f);
    let a = hash01(seed, i as i64);
    let b = hash01(seed, i as i64 + 1);
    a + (b - a) * s
}

/// Terrain height in whole blocks for world column `col`.
fn column_height(seed: u64, col: i64, amplitude: f32) -> i64 {
    let n = noise(seed, col as f32 / 7.0) * 0.7 + noise(seed ^ 0x55, col as f32 / 2.5) * 0.3;
    (n * amplitude).round() as i64
}

fn shade(c: Color32, amount: i32) -> Color32 {
    let f = |v: u8| (i32::from(v) + amount).clamp(0, 255) as u8;
    Color32::from_rgb(f(c.r()), f(c.g()), f(c.b()))
}

struct Layer {
    seed: u64,
    block: f32,
    base: f32,
    amplitude: f32,
    speed: f32,
    grass: Color32,
    dirt: Color32,
    leaves: Color32,
    trees: bool,
}

fn paint_layer(painter: &Painter, rect: Rect, layer: &Layer, time: f64) {
    let bs = layer.block;
    let scroll = time as f32 * layer.speed;
    let first = (scroll / bs).floor() as i64;
    let offset = scroll - first as f32 * bs;
    let columns = (rect.width() / bs).ceil() as i64 + 2;
    let base_y = rect.top() + rect.height() * layer.base;
    let bottom = rect.bottom();
    for i in 0..columns {
        let col = first + i;
        let x = rect.left() + i as f32 * bs - offset;
        let h = column_height(layer.seed, col, layer.amplitude);
        let top = base_y - h as f32 * bs;
        let v = ((hash01(layer.seed ^ 0x9, col) - 0.5) * 14.0) as i32;
        let grass_h = (bs * 0.34).max(2.0);
        painter.rect_filled(
            Rect::from_min_max(pos2(x, top), pos2(x + bs + 0.5, top + grass_h)),
            CornerRadius::ZERO,
            shade(layer.grass, v),
        );
        painter.rect_filled(
            Rect::from_min_max(pos2(x, top + grass_h), pos2(x + bs + 0.5, bottom)),
            CornerRadius::ZERO,
            shade(layer.dirt, v),
        );
        if layer.trees && hash01(layer.seed ^ 0x77, col) > 0.94 {
            let trunk = Color32::from_rgb(92, 64, 40);
            painter.rect_filled(
                Rect::from_min_max(pos2(x, top - 4.0 * bs), pos2(x + bs, top)),
                CornerRadius::ZERO,
                shade(trunk, v),
            );
            for (row, half) in [(-5.0_f32, 2.0_f32), (-4.0, 2.0), (-6.0, 1.0)] {
                painter.rect_filled(
                    Rect::from_min_max(
                        pos2(x - half * bs, top + row * bs),
                        pos2(x + (half + 1.0) * bs, top + (row + 1.0) * bs),
                    ),
                    CornerRadius::ZERO,
                    shade(layer.leaves, v / 2),
                );
            }
        }
    }
}

/// Paint the generated voxel landscape into `rect`. `time` drives the slow drift; pass a
/// constant for a still image.
pub fn paint_landscape(painter: &Painter, rect: Rect, time: f64, accent: Color32) {
    let h = rect.height();
    // Dusk sky.
    let split = rect.top() + h * 0.62;
    gradient_v(
        painter,
        Rect::from_min_max(rect.min, pos2(rect.right(), split)),
        Color32::from_rgb(10, 6, 30),
        Color32::from_rgb(70, 34, 122),
    );
    gradient_v(
        painter,
        Rect::from_min_max(pos2(rect.left(), split), rect.max),
        Color32::from_rgb(70, 34, 122),
        Color32::from_rgb(238, 132, 150),
    );

    // Stars.
    for i in 0..60 {
        let sx = rect.left() + hash01(1, i) * rect.width();
        let sy = rect.top() + hash01(2, i) * h * 0.5;
        let twinkle = if time > 0.0 {
            0.5 + 0.5 * ((time as f32) * 1.3 + i as f32).sin()
        } else {
            0.7
        };
        painter.rect_filled(
            Rect::from_center_size(pos2(sx, sy), Vec2::splat(2.0 + hash01(3, i) * 1.5)),
            CornerRadius::ZERO,
            Color32::from_white_alpha((70.0 + 150.0 * twinkle * hash01(4, i)) as u8),
        );
    }

    // A square sun, as in the blocky world it imitates.
    let sun = Rect::from_center_size(
        pos2(rect.left() + rect.width() * 0.64, rect.top() + h * 0.36),
        Vec2::splat(h * 0.11),
    );
    glow(
        painter,
        sun,
        0,
        Color32::from_rgba_unmultiplied(255, 176, 120, 90),
        60,
    );
    painter.rect_filled(sun, CornerRadius::ZERO, Color32::from_rgb(255, 226, 168));

    // Blocky clouds.
    let cloud_block = h * 0.03;
    for k in 0..7 {
        let lane = (k % 3) as f32;
        let y = rect.top() + h * (0.10 + 0.09 * lane) + hash01(5, k) * h * 0.04;
        let span = rect.width() + 8.0 * cloud_block;
        let x = (hash01(6, k) * span + time as f32 * (5.0 + 3.0 * lane)).rem_euclid(span)
            - 4.0 * cloud_block;
        let w = (3.0 + (hash01(7, k) * 4.0).floor()) * cloud_block;
        let col = Color32::from_rgba_unmultiplied(236, 214, 255, 70);
        painter.rect_filled(
            Rect::from_min_size(pos2(rect.left() + x, y), Vec2::new(w, cloud_block)),
            CornerRadius::ZERO,
            col,
        );
        painter.rect_filled(
            Rect::from_min_size(
                pos2(rect.left() + x + cloud_block, y - cloud_block),
                Vec2::new(w - 2.0 * cloud_block, cloud_block),
            ),
            CornerRadius::ZERO,
            col,
        );
    }

    let seed = hash_str("legohans");
    let layers = [
        Layer {
            seed,
            block: h / 30.0,
            base: 0.70,
            amplitude: 7.0,
            speed: 4.0,
            grass: Color32::from_rgb(70, 46, 124),
            dirt: Color32::from_rgb(48, 30, 88),
            leaves: Color32::from_rgb(58, 40, 108),
            trees: false,
        },
        Layer {
            seed: seed ^ 0x1111,
            block: h / 22.0,
            base: 0.80,
            amplitude: 6.0,
            speed: 9.0,
            grass: Color32::from_rgb(56, 128, 108),
            dirt: Color32::from_rgb(64, 50, 90),
            leaves: Color32::from_rgb(34, 86, 84),
            trees: true,
        },
        Layer {
            seed: seed ^ 0x2222,
            block: h / 15.0,
            base: 0.90,
            amplitude: 4.0,
            speed: 18.0,
            grass: Color32::from_rgb(86, 166, 72),
            dirt: Color32::from_rgb(108, 76, 54),
            leaves: Color32::from_rgb(54, 138, 62),
            trees: false,
        },
    ];
    for layer in &layers {
        paint_layer(painter, rect, layer, time);
    }
    // Tint toward the accent so the art always matches the theme.
    painter.rect_filled(rect, CornerRadius::ZERO, accent.gamma_multiply(0.06));
}

/// Paint a wide, horizontally wrapping panorama strip covering `rect`, slowly panning.
pub fn paint_panorama(painter: &Painter, rect: Rect, texture: &TextureHandle, time: f64) {
    let tex = texture.size_vec2();
    if tex.x <= 0.0 || tex.y <= 0.0 {
        return;
    }
    let mut scale = rect.height() / tex.y;
    if tex.x * scale < rect.width() {
        scale = rect.width() / tex.x;
    }
    let frac_x = (rect.width() / (tex.x * scale)).clamp(0.0, 1.0);
    let frac_y = (rect.height() / (tex.y * scale)).clamp(0.0, 1.0);
    let v0 = (1.0 - frac_y) / 2.0;
    let u0 = ((time * 0.012) % 1.0) as f32;
    let tint = Color32::WHITE;
    let id = texture.id();
    if u0 + frac_x <= 1.0 {
        painter.image(
            id,
            rect,
            Rect::from_min_max(pos2(u0, v0), pos2(u0 + frac_x, v0 + frac_y)),
            tint,
        );
    } else {
        let first = (1.0 - u0) / frac_x;
        let split = rect.left() + rect.width() * first;
        painter.image(
            id,
            Rect::from_min_max(rect.min, Pos2::new(split, rect.bottom())),
            Rect::from_min_max(pos2(u0, v0), pos2(1.0, v0 + frac_y)),
            tint,
        );
        painter.image(
            id,
            Rect::from_min_max(Pos2::new(split, rect.top()), rect.max),
            Rect::from_min_max(pos2(0.0, v0), pos2(frac_x - (1.0 - u0), v0 + frac_y)),
            tint,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noise_is_deterministic_and_bounded() {
        for i in 0..200 {
            let v = noise(42, i as f32 * 0.37);
            assert!((0.0..=1.0).contains(&v));
            assert_eq!(v, noise(42, i as f32 * 0.37));
        }
    }

    #[test]
    fn heights_stay_within_amplitude() {
        for col in -50..50 {
            let h = column_height(7, col, 6.0);
            assert!((0..=6).contains(&h));
        }
    }

    #[test]
    fn landscape_paints_without_panicking() {
        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 500.0));
                paint_landscape(ui.painter(), rect, 12.5, Color32::from_rgb(139, 92, 246));
            });
        });
    }
}
