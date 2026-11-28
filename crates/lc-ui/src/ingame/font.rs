//! Pixel typography backed by the supplied Minecraft ASCII bitmap font.

use egui::{Color32, Painter, Pos2, Rect, TextureHandle, pos2};

use super::assets::{self, DecodedImage};

/// A bitmap font with per-glyph advances measured from the original atlas.
#[derive(Clone)]
pub struct BitmapFont {
    texture: TextureHandle,
    advances: [u8; 256],
}

impl BitmapFont {
    /// Build the font from the bundled `ascii.png`.
    pub fn new(_ctx: &egui::Context, texture: TextureHandle) -> Self {
        let image = assets::decode(assets::FONT).expect("invalid bundled font");
        let advances = measure_advances(&image);
        Self { texture, advances }
    }

    /// Width of `text` at an integer pixel scale.
    pub fn width(&self, text: &str, scale: u32) -> f32 {
        let scale = scale.max(1) as f32;
        text.chars()
            .map(|c| {
                if c == '\n' {
                    0.0
                } else {
                    f32::from(self.advances[glyph_index(c) as usize]) * scale
                }
            })
            .sum()
    }

    /// Advance of one character at an integer pixel scale.
    pub fn advance(&self, ch: char, scale: u32) -> f32 {
        f32::from(self.advances[glyph_index(ch) as usize]) * scale.max(1) as f32
    }

    /// Height of one line at an integer pixel scale.
    pub fn line_height(&self, scale: u32) -> f32 {
        8.0 * scale.max(1) as f32
    }

    /// Draw left-aligned text with the classic one-pixel drop shadow.
    pub fn draw(
        &self,
        painter: &Painter,
        pos: Pos2,
        text: &str,
        scale: u32,
        color: Color32,
        shadow: bool,
    ) {
        let scale = scale.max(1) as f32;
        let mut cursor = pos;
        for ch in text.chars() {
            if ch == '\n' {
                cursor.x = pos.x;
                cursor.y += self.line_height(scale as u32);
                continue;
            }
            let index = glyph_index(ch) as usize;
            let advance = f32::from(self.advances[index]) * scale;
            let uv = glyph_uv(index as u32);
            if shadow {
                let shadow_rect = Rect::from_min_size(
                    cursor + egui::vec2(scale, scale),
                    egui::vec2(8.0 * scale, 8.0 * scale),
                );
                painter.image(
                    self.texture.id(),
                    shadow_rect,
                    uv,
                    Color32::from_rgba_premultiplied(0, 0, 0, 200),
                );
            }
            let rect = Rect::from_min_size(cursor, egui::vec2(8.0 * scale, 8.0 * scale));
            painter.image(self.texture.id(), rect, uv, color);
            cursor.x += advance;
        }
    }

    /// Draw text centered in `rect`.
    pub fn draw_centered(
        &self,
        painter: &Painter,
        rect: Rect,
        text: &str,
        scale: u32,
        color: Color32,
        shadow: bool,
    ) {
        let width = self.width(text, scale);
        let height = self.line_height(scale);
        self.draw(
            painter,
            pos2(
                rect.center().x - width / 2.0,
                rect.center().y - height / 2.0,
            ),
            text,
            scale,
            color,
            shadow,
        );
    }
}

/// Map a character to a cell in the supplied atlas. Unsupported characters become `?`.
pub fn glyph_index(ch: char) -> u32 {
    let code = ch as u32;
    // `ascii.png` starts its printable range at cell 32 (row 2, column 0), so the cell index
    // for printable ASCII is the Unicode code point itself.
    if (0x20..0x7f).contains(&code) {
        code
    } else {
        u32::from(b'?')
    }
}

/// UV rectangle for a glyph cell, matching Minecraft's 16x16 layout.
pub fn glyph_uv(index: u32) -> Rect {
    let col = index % 16;
    let row = index / 16;
    Rect::from_min_max(
        pos2(col as f32 / 16.0, row as f32 / 16.0),
        pos2((col + 1) as f32 / 16.0, (row + 1) as f32 / 16.0),
    )
}

/// Advance widths measured from the non-transparent pixels of each atlas cell.
pub fn measure_advances(image: &DecodedImage) -> [u8; 256] {
    let mut advances = [6u8; 256];
    advances[b' ' as usize] = 4;
    let cell = 8usize;
    for code in 0x21u8..0x7f {
        let index = glyph_index(char::from(code)) as usize;
        let col = index % 16;
        let row = index / 16;
        let mut last = 0usize;
        for y in 0..cell {
            for x in 0..cell {
                let px = (col * cell + x) + (row * cell + y) * image.width;
                if px < image.rgba.len() / 4 && image.rgba[px * 4 + 3] > 24 {
                    last = last.max(x + 1);
                }
            }
        }
        advances[code as usize] = last.clamp(1, 8) as u8;
    }
    advances
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_chars_use_the_expected_atlas_rows() {
        assert_eq!(glyph_index(' '), 32);
        assert_eq!(glyph_index('0'), 48);
        assert_eq!(glyph_index('A'), 65);
        assert_eq!(glyph_index('a'), 97);
        assert_eq!(glyph_index('é'), glyph_index('?'));
    }

    #[test]
    fn measured_advances_are_pixel_sized() {
        let image = assets::decode(assets::FONT).unwrap();
        let advances = measure_advances(&image);
        assert!(advances[b' ' as usize] >= 4);
        assert!((1..=8).contains(&advances[b'A' as usize]));
        assert!((1..=8).contains(&advances[b'W' as usize]));
    }
}
