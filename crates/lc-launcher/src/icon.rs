//! Shared 64x64 launcher icon artwork.
//!
//! The same pixels are used for the eframe window icon and, via `lc-app/build.rs`, for the
//! Windows executable resource. The artwork is a black isometric cube with a light "LC" mark.

/// Icon width and height in pixels.
pub const SIZE: usize = 64;

type Quad = [(f32, f32); 4];

fn cube_faces() -> [(Quad, [u8; 3]); 3] {
    let c = SIZE as f32 / 2.0;
    let r = c - 3.0;
    let p = |x: f32, y: f32| (c + x * r, c + y * r);
    [
        (
            [p(0.0, -1.0), p(0.866, -0.5), p(0.0, 0.0), p(-0.866, -0.5)],
            [30, 30, 38],
        ),
        (
            [p(-0.866, -0.5), p(0.0, 0.0), p(0.0, 1.0), p(-0.866, 0.5)],
            [8, 8, 13],
        ),
        (
            [p(0.0, 0.0), p(0.866, -0.5), p(0.866, 0.5), p(0.0, 1.0)],
            [17, 17, 24],
        ),
    ]
}

fn inside(poly: &Quad, x: f32, y: f32) -> bool {
    let mut sign = 0.0f32;
    for i in 0..4 {
        let (ax, ay) = poly[i];
        let (bx, by) = poly[(i + 1) % 4];
        let cross = (bx - ax) * (y - ay) - (by - ay) * (x - ax);
        if cross != 0.0 {
            if sign == 0.0 {
                sign = cross.signum();
            } else if sign != cross.signum() {
                return false;
            }
        }
    }
    true
}

// 5x7 bitmap letters, one byte per row with the least significant bit on the left.
const GLYPHS: [[u8; 7]; 2] = [
    [0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x1f], // L
    [0x0e, 0x11, 0x10, 0x10, 0x10, 0x11, 0x0e], // C
];

fn mark_pixel(x: usize, y: usize) -> bool {
    const SCALE: usize = 2;
    const START_X: usize = 34;
    const START_Y: usize = 35;
    if !(x >= START_X && y >= START_Y) {
        return false;
    }
    let local_x = x - START_X;
    let local_y = y - START_Y;
    let glyph = local_x / (6 * SCALE);
    if glyph >= 2 {
        return false;
    }
    let in_glyph_x = local_x % (6 * SCALE);
    let col = in_glyph_x / SCALE;
    let row = local_y / SCALE;
    if col >= 5 || row >= 7 {
        return false;
    }
    GLYPHS[glyph][row] & (1 << (4 - col)) != 0
}

/// Build the RGBA8 icon pixels.
pub fn rgba() -> Vec<u8> {
    let mut rgba = vec![0u8; SIZE * SIZE * 4];
    let faces = cube_faces();
    for y in 0..SIZE {
        for x in 0..SIZE {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let mut color = None;
            for (poly, face_color) in &faces {
                if inside(poly, px, py) {
                    color = Some(*face_color);
                }
            }
            let Some(mut color) = color else {
                continue;
            };
            if mark_pixel(x, y) {
                color = [236, 234, 248];
            }
            let i = (y * SIZE + x) * 4;
            rgba[i..i + 4].copy_from_slice(&[color[0], color[1], color[2], 255]);
        }
    }
    rgba
}

/// Build the 1-bit AND mask used by a 32-bit Windows ICO. Rows are bottom-up, MSB first and
/// padded to a four-byte boundary; a set bit means the pixel is transparent.
// Used by `lc-app/build.rs`; the launcher library itself only needs it for the test below.
#[allow(dead_code)]
pub fn ico_and_mask(rgba: &[u8], size: usize) -> Vec<u8> {
    let stride = size.div_ceil(32) * 4;
    let mut mask = vec![0u8; stride * size];
    for y in (0..size).rev() {
        let row = (size - 1 - y) * stride;
        for x in 0..size {
            if rgba[(y * size + x) * 4 + 3] == 0 {
                mask[row + x / 8] |= 0x80 >> (x % 8);
            }
        }
    }
    mask
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_is_a_black_cube_with_a_mark() {
        let rgba = rgba();
        assert_eq!(rgba.len(), SIZE * SIZE * 4);
        assert_eq!(rgba[3], 0, "the corners stay transparent");
        let opaque = rgba.chunks_exact(4).filter(|p| p[3] == 255).count();
        assert!(opaque > 1000, "the cube should cover a large area");
        let dark = rgba
            .chunks_exact(4)
            .filter(|p| p[3] == 255 && p[0] < 40 && p[1] < 40 && p[2] < 48)
            .count();
        let bright = rgba
            .chunks_exact(4)
            .filter(|p| p[3] == 255 && p[0] > 200 && p[1] > 200)
            .count();
        assert!(dark > 1000, "the cube faces must be nearly black");
        assert!(bright > 40, "the LC mark must be visible");
    }

    #[test]
    fn ico_mask_marks_transparent_pixels_only() {
        let rgba = rgba();
        let mask = ico_and_mask(&rgba, SIZE);
        let stride = SIZE.div_ceil(32) * 4;
        assert_eq!(mask.len(), stride * SIZE);
        assert_eq!(mask[0] & 0x80, 0x80, "the top-left corner is transparent");
        let center_row = (SIZE / 2) * stride;
        assert_eq!(
            mask[center_row + (SIZE / 2) / 8] & (0x80 >> ((SIZE / 2) % 8)),
            0,
            "the cube center is opaque"
        );
    }
}
