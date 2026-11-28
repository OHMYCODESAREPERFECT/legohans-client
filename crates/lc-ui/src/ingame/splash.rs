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

/// A heavy outlined wordmark, used for the custom edition label under the logo.
#[derive(Clone)]
pub struct WordmarkText {
    /// Texture containing the outlined wordmark.
    pub texture: TextureHandle,
    /// Texture width in screen points.
    pub width: f32,
    /// Texture height in screen points.
    pub height: f32,
}

impl WordmarkText {
    /// Render `text` with a thick black outline, gray fill and a dark drop shadow.
    ///
    /// `stretch` widens the supplied bitmap font without making the glyphs taller.
    pub fn new(ctx: &Context, font: &BitmapFont, text: &str, scale: u32, stretch: f32) -> Self {
        let scale = scale.max(1);
        let stretch = stretch.clamp(0.5, 2.0);
        let atlas = assets::decode(assets::FONT).expect("invalid bundled font");
        let padding = (scale * 2) as usize;
        let block_w = (scale as f32 * stretch).round().max(1.0) as usize;
        let block_h = scale as usize;
        let width = (font.width(text, scale) * stretch).ceil() as usize + padding * 2;
        let height = block_h * 8 + padding * 2;
        let mut canvas = PixelCanvas::new(width, height);

        let mut glyphs = Vec::new();
        let mut cursor = padding as f32;
        for ch in text.chars() {
            let index = glyph_index(ch) as usize;
            let col = index % 16;
            let row = index / 16;
            for gy in 0..8usize {
                for gx in 0..8usize {
                    let src = ((row * 8 + gy) * atlas.width + col * 8 + gx) * 4;
                    if atlas.rgba.get(src + 3).copied().unwrap_or(0) <= 24 {
                        continue;
                    }
                    let x = (cursor + gx as f32 * scale as f32 * stretch).round() as usize;
                    let y = padding + gy * block_h;
                    glyphs.push((x, y));
                }
            }
            cursor += font.advance(ch, scale) * stretch;
        }

        let shadow = scale as isize;
        let outline = 1usize.max(scale as usize / 2);
        let shadow_color = Color32::from_rgb(72, 72, 72);
        let outline_color = Color32::from_rgb(0, 0, 0);
        let fill = Color32::from_rgb(220, 220, 214);
        let highlight = Color32::from_rgb(246, 246, 240);

        for &(x, y) in &glyphs {
            // Drop shadow, offset down-right.
            canvas.fill(
                x as isize + shadow,
                y as isize + shadow,
                block_w,
                block_h,
                shadow_color,
            );
            // Heavy outline.
            canvas.fill(
                x as isize - outline as isize,
                y as isize - outline as isize,
                block_w + outline * 2,
                block_h + outline * 2,
                outline_color,
            );
            // Gray face with a light top-left bevel.
            canvas.fill(x as isize, y as isize, block_w, block_h, fill);
            canvas.fill(
                x as isize,
                y as isize,
                block_w.max(2) / 2,
                block_h.max(2) / 2,
                highlight,
            );
        }

        let image = egui::ColorImage::new([width, height], canvas.pixels);
        let texture = ctx.load_texture("ingame-edition", image, TextureOptions::NEAREST);
        Self {
            texture,
            width: width as f32,
            height: height as f32,
        }
    }
}

/// Small nearest-neighbour drawing surface used while baking wordmarks.
struct PixelCanvas {
    pixels: Vec<Color32>,
    width: usize,
    height: usize,
}

impl PixelCanvas {
    fn new(width: usize, height: usize) -> Self {
        Self {
            pixels: vec![Color32::TRANSPARENT; width * height],
            width,
            height,
        }
    }

    fn fill(&mut self, x: isize, y: isize, w: usize, h: usize, color: Color32) {
        for dy in 0..h as isize {
            for dx in 0..w as isize {
                let px = x + dx;
                let py = y + dy;
                if px >= 0 && py >= 0 && (px as usize) < self.width && (py as usize) < self.height {
                    self.pixels[py as usize * self.width + px as usize] = color;
                }
            }
        }
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
