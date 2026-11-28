//! The bouncing yellow phrase drawn across the main-menu logo.

use egui::{Color32, Context, TextureHandle, TextureOptions};

use super::{
    assets,
    font::{BitmapFont, glyph_index},
};

/// A pixel-font phrase rendered once into a transparent, nearest-filtered texture.
#[derive(Clone)]
pub struct SplashText {
    /// Texture containing the outlined phrase.
    pub texture: TextureHandle,
    /// Width of the phrase in screen points.
    pub width: f32,
    /// Height of the phrase in screen points.
    pub height: f32,
}

impl SplashText {
    /// Render `phrase` with the supplied bitmap font at an integer pixel scale.
    pub fn new(ctx: &Context, font: &BitmapFont, phrase: &str, scale: u32) -> Self {
        let scale = scale.max(1);
        let atlas = assets::decode(assets::FONT).expect("invalid bundled font");
        let padding = scale as usize;
        let width = font.width(phrase, scale).ceil() as usize + padding * 2;
        let height = (8 * scale) as usize + padding * 2;
        let mut pixels = vec![Color32::TRANSPARENT; width * height];

        // Collect the opaque glyph pixels before adding the outline, so neighbouring glyphs do
        // not erase each other's borders.
        let mut glyph_pixels = Vec::new();
        let mut cursor = padding;
        for ch in phrase.chars() {
            let index = glyph_index(ch) as usize;
            let col = index % 16;
            let row = index / 16;
            for gy in 0..8usize {
                for gx in 0..8usize {
                    let src = ((row * 8 + gy) * atlas.width + col * 8 + gx) * 4;
                    if atlas.rgba.get(src + 3).copied().unwrap_or(0) <= 24 {
                        continue;
                    }
                    glyph_pixels
                        .push((cursor + gx * scale as usize, padding + gy * scale as usize));
                }
            }
            cursor += font.advance(ch, scale).round().max(1.0) as usize;
        }

        let outline = 1usize.max(scale as usize / 3);
        for &(x, y) in &glyph_pixels {
            for oy in -(outline as isize)..(scale as isize + outline as isize) {
                for ox in -(outline as isize)..(scale as isize + outline as isize) {
                    let px = x as isize + ox;
                    let py = y as isize + oy;
                    if px >= 0 && py >= 0 && (px as usize) < width && (py as usize) < height {
                        pixels[py as usize * width + px as usize] = Color32::from_rgb(0, 0, 0);
                    }
                }
            }
        }
        let yellow = Color32::from_rgb(255, 214, 32);
        for (x, y) in glyph_pixels {
            for dy in 0..scale as usize {
                for dx in 0..scale as usize {
                    let px = x + dx;
                    let py = y + dy;
                    if px < width && py < height {
                        pixels[py * width + px] = yellow;
                    }
                }
            }
        }

        let image = egui::ColorImage::new([width, height], pixels);
        let texture = ctx.load_texture("ingame-splash", image, TextureOptions::NEAREST);
        Self {
            texture,
            width: width as f32,
            height: height as f32,
        }
    }

    /// Rotation used by the vanilla-looking title splash.
    pub fn angle() -> f32 {
        -0.34
    }

    /// Vertical bounce offset for a time in seconds.
    pub fn bounce(time: f32) -> f32 {
        (time * 2.1).sin() * 6.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounce_stays_within_a_small_range() {
        for i in 0..100 {
            let t = i as f32 * 0.1;
            assert!(SplashText::bounce(t).abs() <= 6.0);
        }
    }
}
