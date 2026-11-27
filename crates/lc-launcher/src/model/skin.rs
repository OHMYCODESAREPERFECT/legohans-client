//! Validation and import of Minecraft skin, cape and elytra textures.

use std::fs;
use std::io;
use std::path::Path;

use crate::services::images::{decode_png, png_dimensions};

/// Largest file accepted for a customization texture.
const MAX_TEXTURE_BYTES: u64 = 8 * 1024 * 1024;

/// What a PNG is being imported as.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureKind {
    /// A player skin (64x64 modern or 64x32 legacy).
    Skin,
    /// A cape (64x32).
    Cape,
    /// An elytra texture (64x32).
    Elytra,
}

impl TextureKind {
    /// Human-readable name used in error messages.
    pub fn label(self) -> &'static str {
        match self {
            Self::Skin => "skin",
            Self::Cape => "cape",
            Self::Elytra => "elytra",
        }
    }
}

/// Dimensions and format of an accepted texture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextureInfo {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Whether this is a legacy 64x32 skin.
    pub legacy: bool,
}

/// Decode a texture far enough to verify that it is a real PNG, not just a valid header.
fn dimensions(bytes: &[u8]) -> Result<(u32, u32), String> {
    let (width, height) = png_dimensions(bytes)?;
    // Only textures at most 64x64 can be accepted. Avoid decoding a huge, highly compressible
    // image just to report its dimensions; decode smaller candidates fully so corrupt pixel data
    // cannot masquerade as a valid skin.
    if width > 64 || height > 64 {
        return Ok((width, height));
    }
    let image = decode_png(bytes, u32::MAX).map_err(|e| format!("Invalid PNG: {e}"))?;
    Ok((image.width as u32, image.height as u32))
}

/// Validate skin PNG bytes. Accepts the modern 64x64 format and the legacy 64x32 format.
pub fn validate_skin(bytes: &[u8]) -> Result<TextureInfo, String> {
    let (width, height) = dimensions(bytes)?;
    match (width, height) {
        (64, 64) => Ok(TextureInfo {
            width,
            height,
            legacy: false,
        }),
        (64, 32) => Ok(TextureInfo {
            width,
            height,
            legacy: true,
        }),
        _ => Err(format!(
            "A skin must be 64x64 or 64x32 pixels, but this image is {width}x{height}."
        )),
    }
}

/// Validate cape PNG bytes. Minecraft capes use a 64x32 texture.
pub fn validate_cape(bytes: &[u8]) -> Result<TextureInfo, String> {
    validate_64x32(bytes, "cape")
}

/// Validate elytra PNG bytes. Minecraft elytra use a 64x32 texture.
pub fn validate_elytra(bytes: &[u8]) -> Result<TextureInfo, String> {
    validate_64x32(bytes, "elytra")
}

fn validate_64x32(bytes: &[u8], label: &str) -> Result<TextureInfo, String> {
    let (width, height) = dimensions(bytes)?;
    if (width, height) != (64, 32) {
        return Err(format!(
            "A {label} texture must be 64x32 pixels, but this image is {width}x{height}."
        ));
    }
    Ok(TextureInfo {
        width,
        height,
        legacy: false,
    })
}

/// Validate `bytes` for `kind`.
pub fn validate(bytes: &[u8], kind: TextureKind) -> Result<TextureInfo, String> {
    match kind {
        TextureKind::Skin => validate_skin(bytes),
        TextureKind::Cape => validate_cape(bytes),
        TextureKind::Elytra => validate_elytra(bytes),
    }
}

/// Read and validate a texture file without copying it.
pub fn read_texture(path: &Path, kind: TextureKind) -> Result<Vec<u8>, String> {
    let meta = fs::metadata(path).map_err(|e| format!("Could not read {}: {e}", path.display()))?;
    if !meta.is_file() {
        return Err(format!("{} is not a file.", path.display()));
    }
    if meta.len() > MAX_TEXTURE_BYTES {
        return Err(format!(
            "{} is too large to be a {} texture.",
            path.display(),
            kind.label()
        ));
    }
    let bytes = fs::read(path).map_err(|e| format!("Could not read {}: {e}", path.display()))?;
    validate(&bytes, kind)?;
    Ok(bytes)
}

/// Validate `source` and copy it to `destination`.
pub fn import_texture(
    source: &Path,
    destination: &Path,
    kind: TextureKind,
) -> Result<TextureInfo, String> {
    let bytes = read_texture(source, kind)?;
    let info = validate(&bytes, kind)?;
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Could not create {}: {e}", parent.display()))?;
    }
    write_atomic(destination, &bytes)
        .map_err(|e| format!("Could not save {}: {e}", destination.display()))?;
    Ok(info)
}

fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let tmp = path.with_extension("png.tmp");
    fs::write(&tmp, bytes)?;
    fs::rename(&tmp, path)
}

/// Whether a usable texture file exists at `path` for `kind`.
pub fn texture_exists(path: &Path, kind: TextureKind) -> bool {
    match fs::read(path) {
        Ok(bytes) => validate(&bytes, kind).is_ok(),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{TempDir, encode_png};

    fn png(w: u32, h: u32) -> Vec<u8> {
        encode_png(w, h, &[40u8, 80, 120, 255].repeat((w * h) as usize))
    }

    #[test]
    fn accepts_modern_and_legacy_skins() {
        assert!(!validate_skin(&png(64, 64)).unwrap().legacy);
        assert!(validate_skin(&png(64, 32)).unwrap().legacy);
    }

    #[test]
    fn rejects_wrong_skin_dimensions() {
        let mut truncated = png(64, 64);
        truncated.truncate(truncated.len() / 2);
        assert!(
            validate_skin(&truncated)
                .unwrap_err()
                .contains("Invalid PNG")
        );
        assert!(validate_skin(&png(64, 48)).unwrap_err().contains("64x64"));
        assert!(
            validate_skin(&png(128, 128))
                .unwrap_err()
                .contains("128x128")
        );
        assert!(validate_skin(b"not a png").is_err());
    }

    #[test]
    fn cape_and_elytra_require_64x32() {
        assert!(validate_cape(&png(64, 32)).is_ok());
        assert!(validate_elytra(&png(64, 32)).is_ok());
        assert!(validate_cape(&png(64, 64)).is_err());
        assert!(validate_elytra(&png(32, 64)).is_err());
    }

    #[test]
    fn import_copies_only_valid_textures() {
        let dir = TempDir::new();
        let source = dir.path().join("in.png");
        let destination = dir.path().join("custom").join("skin.png");
        fs::write(&source, png(64, 64)).unwrap();
        let info = import_texture(&source, &destination, TextureKind::Skin).unwrap();
        assert_eq!((info.width, info.height), (64, 64));
        assert!(texture_exists(&destination, TextureKind::Skin));

        fs::write(&source, png(20, 20)).unwrap();
        assert!(import_texture(&source, &destination, TextureKind::Skin).is_err());
        assert!(texture_exists(&destination, TextureKind::Skin));
    }

    #[test]
    fn missing_files_are_reported() {
        let dir = TempDir::new();
        assert!(read_texture(&dir.path().join("nope.png"), TextureKind::Skin).is_err());
        assert!(!texture_exists(
            &dir.path().join("nope.png"),
            TextureKind::Skin
        ));
    }
}
