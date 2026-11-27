//! Small color helpers used by the custom accent picker.

/// Hue, saturation and value. `h` is in degrees, `s` and `v` are in `0..=1`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hsv {
    /// Hue in degrees.
    pub h: f32,
    /// Saturation.
    pub s: f32,
    /// Value (brightness).
    pub v: f32,
}

impl Hsv {
    /// Convert RGB bytes to HSV.
    pub fn from_rgb([r, g, b]: [u8; 3]) -> Self {
        let (r, g, b) = (
            f32::from(r) / 255.0,
            f32::from(g) / 255.0,
            f32::from(b) / 255.0,
        );
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let delta = max - min;
        let h = if delta <= f32::EPSILON {
            0.0
        } else if max == r {
            60.0 * (((g - b) / delta) % 6.0)
        } else if max == g {
            60.0 * ((b - r) / delta + 2.0)
        } else {
            60.0 * ((r - g) / delta + 4.0)
        };
        Self {
            h: if h < 0.0 { h + 360.0 } else { h },
            s: if max <= f32::EPSILON {
                0.0
            } else {
                delta / max
            },
            v: max,
        }
    }

    /// Convert HSV to RGB bytes.
    pub fn to_rgb(self) -> [u8; 3] {
        let h = self.h.rem_euclid(360.0) / 60.0;
        let s = self.s.clamp(0.0, 1.0);
        let v = self.v.clamp(0.0, 1.0);
        let c = v * s;
        let x = c * (1.0 - (h % 2.0 - 1.0).abs());
        let m = v - c;
        let (r, g, b) = match h as u32 {
            0 => (c, x, 0.0),
            1 => (x, c, 0.0),
            2 => (0.0, c, x),
            3 => (0.0, x, c),
            4 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };
        [
            ((r + m) * 255.0).round().clamp(0.0, 255.0) as u8,
            ((g + m) * 255.0).round().clamp(0.0, 255.0) as u8,
            ((b + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        ]
    }
}

/// Parse `#RRGGBB` or `RRGGBB` into RGB bytes.
pub fn parse_hex_rgb(input: &str) -> Result<[u8; 3], String> {
    let text = input.trim().trim_start_matches('#');
    if text.len() != 6 || !text.bytes().all(|c| c.is_ascii_hexdigit()) {
        return Err("Use a six-digit hex color, for example #8B5CF6.".to_string());
    }
    let channel =
        |start: usize| -> u8 { u8::from_str_radix(&text[start..start + 2], 16).unwrap_or(0) };
    Ok([channel(0), channel(2), channel(4)])
}

/// Format RGB bytes as `#RRGGBB`.
pub fn format_hex_rgb([r, g, b]: [u8; 3]) -> String {
    format!("#{r:02X}{g:02X}{b:02X}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hsv_roundtrips_common_colors() {
        for rgb in [
            [0, 0, 0],
            [255, 255, 255],
            [255, 0, 0],
            [0, 255, 0],
            [0, 0, 255],
            [139, 92, 246],
            [12, 200, 99],
        ] {
            assert_eq!(Hsv::from_rgb(rgb).to_rgb(), rgb);
        }
    }

    #[test]
    fn hsv_clamps_out_of_range_values() {
        let hsv = Hsv {
            h: -30.0,
            s: 2.0,
            v: -1.0,
        };
        assert_eq!(hsv.to_rgb(), [0, 0, 0]);
        assert_eq!(
            Hsv {
                h: 390.0,
                s: 1.0,
                v: 1.0
            }
            .to_rgb(),
            [255, 128, 0]
        );
    }

    #[test]
    fn hex_colors_are_parsed_and_formatted() {
        assert_eq!(parse_hex_rgb("#8B5CF6"), Ok([139, 92, 246]));
        assert_eq!(parse_hex_rgb("0a0B0c"), Ok([10, 11, 12]));
        assert!(parse_hex_rgb("#fff").is_err());
        assert!(parse_hex_rgb("#GGGGGG").is_err());
        assert_eq!(format_hex_rgb([139, 92, 246]), "#8B5CF6");
    }
}
