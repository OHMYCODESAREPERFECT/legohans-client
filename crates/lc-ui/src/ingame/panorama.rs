//! CPU cube-map projection for the six supplied Java Edition panorama faces.

use egui::{Color32, ColorImage, Context, Painter, Rect, TextureHandle, TextureOptions, Vec2};

use super::assets::{self, DecodedImage};

/// Java Edition face order: south, west, north, east, down, up.
pub const FACE_NAMES: [&str; 6] = [
    "panorama_0 (south)",
    "panorama_1 (west)",
    "panorama_2 (north)",
    "panorama_3 (east)",
    "panorama_4 (down)",
    "panorama_5 (up)",
];

/// Map a cube-space direction to a panorama face and source UV.
pub fn cube_uv(dir: [f32; 3]) -> (usize, f32, f32) {
    let [x, y, z] = dir;
    let ax = x.abs();
    let ay = y.abs();
    let az = z.abs();
    let clamp = |v: f32| v.clamp(0.0, 1.0);
    if ax >= ay && ax >= az {
        if x >= 0.0 {
            (3, clamp((1.0 + z) / 2.0), clamp((1.0 - y) / 2.0))
        } else {
            (1, clamp((1.0 - z) / 2.0), clamp((1.0 - y) / 2.0))
        }
    } else if ay >= az {
        if y >= 0.0 {
            (5, clamp((1.0 - z) / 2.0), clamp((1.0 + x) / 2.0))
        } else {
            (4, clamp((1.0 + z) / 2.0), clamp((1.0 - x) / 2.0))
        }
    } else if z >= 0.0 {
        (0, clamp((1.0 - x) / 2.0), clamp((1.0 - y) / 2.0))
    } else {
        (2, clamp((1.0 + x) / 2.0), clamp((1.0 - y) / 2.0))
    }
}

/// A dynamically re-rendered cube panorama.
pub struct Panorama {
    faces: [DecodedImage; 6],
    texture: TextureHandle,
    image: ColorImage,
}

impl Panorama {
    /// Decode all six supplied faces and upload an initial frame.
    pub fn new(ctx: &Context) -> Self {
        let faces = assets::PANORAMA
            .map(|bytes| assets::decode(bytes).expect("invalid bundled panorama face"));
        let image = ColorImage::new([480, 270], vec![Color32::BLACK; 480 * 270]);
        let texture = ctx.load_texture("ingame-panorama", image.clone(), TextureOptions::LINEAR);
        Self {
            faces,
            texture,
            image,
        }
    }

    /// Number of decoded faces; used by asset tests.
    pub fn face_count(&self) -> usize {
        self.faces.len()
    }

    /// Re-render the cube projection for a yaw/pitch in radians.
    pub fn render(&mut self, yaw: f32, pitch: f32, viewport: Vec2) {
        let width = ((viewport.x / 2.0).round() as usize).clamp(160, 480);
        let height = (width as f32 * viewport.y / viewport.x).round().max(1.0) as usize;
        let height = height.clamp(90, 270);
        if self.image.size != [width, height] {
            self.image = ColorImage::new([width, height], vec![Color32::BLACK; width * height]);
        }
        let (sy, cy) = yaw.sin_cos();
        let (sp, cp) = pitch.sin_cos();
        let focal = width as f32 / 2.0;
        let pixels = &mut self.image.pixels;
        for py in 0..height {
            let screen_y = (height as f32 / 2.0 - py as f32 - 0.5) / focal;
            for px in 0..width {
                let screen_x = (px as f32 + 0.5 - width as f32 / 2.0) / focal;
                // Yaw 0 looks toward +Z (face 0); increasing yaw moves through faces 0,1,2,3.
                let rx = screen_x * cy - sy;
                let rz = screen_x * sy + cy;
                let ry = screen_y * cp + rz * sp;
                let rz2 = -screen_y * sp + rz * cp;
                let (face, u, v) = cube_uv([rx, ry, rz2]);
                let image = &self.faces[face];
                let sx = ((u * image.width as f32) as usize).min(image.width - 1);
                let sy = ((v * image.height as f32) as usize).min(image.height - 1);
                let src = (sy * image.width + sx) * 4;
                let dst = py * width + px;
                pixels[dst] =
                    Color32::from_rgb(image.rgba[src], image.rgba[src + 1], image.rgba[src + 2]);
            }
        }
        self.texture.set(self.image.clone(), TextureOptions::LINEAR);
    }

    /// Paint the panorama and its supplied overlay over `rect`.
    pub fn paint(&self, painter: &Painter, rect: Rect, overlay: &TextureHandle) {
        let full = Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
        painter.image(self.texture.id(), rect, full, Color32::WHITE);
        painter.image(overlay.id(), rect, full, Color32::WHITE);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn near(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }

    #[test]
    fn cardinal_directions_select_the_expected_faces() {
        let cases = [
            ([0.0, 0.0, 1.0], 0),
            ([-1.0, 0.0, 0.0], 1),
            ([0.0, 0.0, -1.0], 2),
            ([1.0, 0.0, 0.0], 3),
            ([0.0, -1.0, 0.0], 4),
            ([0.0, 1.0, 0.0], 5),
        ];
        for (dir, face) in cases {
            let (got_face, u, v) = cube_uv(dir);
            assert_eq!(got_face, face);
            assert!(near(u, 0.5) && near(v, 0.5), "face {face} at {u},{v}");
        }
    }

    #[test]
    fn horizontal_seams_have_matching_uv_edges() {
        // Face 0's right edge (u=1) continues into face 1's left edge (u=0). This is the
        // direct 0,1,2,3 face ordering confirmed by the source-image seam measurements.
        let (f0, u0, v) = cube_uv([-0.999, 0.25, 1.0]);
        let (f1, u1, v1) = cube_uv([-1.0, 0.25, 0.999]);
        assert_eq!((f0, f1), (0, 1));
        assert!(u0 > 0.999 && u1 < 0.001, "seam UVs {u0}, {u1}");
        assert!(near(v, v1));
    }

    #[test]
    fn all_six_faces_decode_as_square_textures() {
        for bytes in assets::PANORAMA {
            let image = assets::decode(bytes).unwrap();
            assert_eq!(image.width, 1024);
            assert_eq!(image.height, 1024);
            assert_eq!(image.rgba.len(), 1024 * 1024 * 4);
        }
    }

    #[test]
    fn projection_is_visible_and_changes_with_yaw() {
        let ctx = Context::default();
        let mut panorama = Panorama::new(&ctx);
        let viewport = Vec2::new(640.0, 360.0);
        panorama.render(0.0, 0.0, viewport);
        let first = panorama.image.pixels.clone();
        assert!(
            first
                .iter()
                .any(|pixel| pixel.r() > 10 || pixel.g() > 10 || pixel.b() > 10),
            "the panorama must not render black"
        );
        panorama.render(std::f32::consts::FRAC_PI_2, 0.0, viewport);
        let changed = panorama
            .image
            .pixels
            .iter()
            .zip(first.iter())
            .filter(|(a, b)| a.r() != b.r() || a.g() != b.g() || a.b() != b.b())
            .count();
        assert!(
            changed > 1000,
            "rotating should change many pixels, got {changed}"
        );
    }
}
