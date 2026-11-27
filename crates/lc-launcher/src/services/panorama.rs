//! Loading title-screen panoramas from the user's own files.
//!
//! The launcher never ships Mojang artwork. The four horizontal panorama faces are read from
//! the user's own Minecraft client jar or from four images imported into the launcher folder.

use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

use super::images::{DecodedImage, decode_png, png_dimensions};

const FACE_LIMIT: u64 = 16 * 1024 * 1024;
const MIN_FACE_DIM: u32 = 64;
const MAX_FACE_DIM: u32 = 4096;

/// Path of the client jar for `version` inside a `.minecraft` folder, if it exists.
pub fn find_client_jar(minecraft_dir: &Path, version: &str) -> Option<PathBuf> {
    let jar = minecraft_dir
        .join("versions")
        .join(version)
        .join(format!("{version}.jar"));
    jar.is_file().then_some(jar)
}

/// Read the four side faces (`panorama_0` to `panorama_3`) from `jar` and join them into one
/// wide strip, each face shrunk to at most `max_face_dim` pixels.
pub fn load_panorama(jar: &Path, max_face_dim: u32) -> Result<DecodedImage, String> {
    let mut archive = zip::ZipArchive::new(File::open(jar).map_err(|e| e.to_string())?)
        .map_err(|e| format!("not a readable jar: {e}"))?;
    let mut faces = Vec::with_capacity(4);
    for i in 0..4 {
        let name = format!("assets/minecraft/textures/gui/title/background/panorama_{i}.png");
        let mut bytes = Vec::new();
        archive
            .by_name(&name)
            .map_err(|_| format!("{name} not found in the jar"))?
            .take(FACE_LIMIT)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        faces.push(decode_png(&bytes, max_face_dim)?);
    }
    stitch_faces(&faces)
}

/// Filename of custom panorama face `index` inside `dir`.
pub fn face_path(dir: &Path, index: usize) -> PathBuf {
    dir.join(format!("panorama_{index}.png"))
}

/// Read one custom face from disk, with a size guard.
pub fn read_face(path: &Path) -> Result<Vec<u8>, String> {
    let meta = fs::metadata(path).map_err(|e| format!("Could not read {}: {e}", path.display()))?;
    if !meta.is_file() {
        return Err(format!("{} is not a file.", path.display()));
    }
    if meta.len() > FACE_LIMIT {
        return Err(format!(
            "{} is too large to be a panorama face.",
            path.display()
        ));
    }
    fs::read(path).map_err(|e| format!("Could not read {}: {e}", path.display()))
}

/// Check that four PNG face images form a compatible panorama. Faces must be square, identical
/// in size, and between 64 and 4096 pixels wide.
pub fn validate_face_bytes(faces: &[Vec<u8>; 4]) -> Result<(u32, u32), String> {
    let mut dimensions = Vec::with_capacity(4);
    for (i, bytes) in faces.iter().enumerate() {
        let (w, h) = png_dimensions(bytes).map_err(|e| format!("Face {}: {e}", i + 1))?;
        if w != h {
            return Err(format!(
                "Face {} is {w}x{h}; panorama faces must be square.",
                i + 1
            ));
        }
        if !(MIN_FACE_DIM..=MAX_FACE_DIM).contains(&w) {
            return Err(format!(
                "Face {} is {w}x{h}; each face must be between {MIN_FACE_DIM} and {MAX_FACE_DIM} pixels.",
                i + 1
            ));
        }
        dimensions.push((w, h));
    }
    if dimensions.iter().any(|d| *d != dimensions[0]) {
        let sizes: Vec<String> = dimensions.iter().map(|(w, h)| format!("{w}x{h}")).collect();
        return Err(format!(
            "Panorama faces must all be the same size (found {}).",
            sizes.join(", ")
        ));
    }
    Ok(dimensions[0])
}

/// Validate four source images and copy them to `dir` as `panorama_0.png` … `panorama_3.png`.
/// Existing files are left untouched if any source is invalid.
pub fn import_custom_panorama(sources: &[PathBuf; 4], dir: &Path) -> Result<(u32, u32), String> {
    let mut bytes = Vec::with_capacity(4);
    for source in sources {
        bytes.push(read_face(source)?);
    }
    let array: [Vec<u8>; 4] = bytes
        .try_into()
        .map_err(|_| "Exactly four faces are required.".to_string())?;
    let dims = validate_face_bytes(&array)?;
    fs::create_dir_all(dir).map_err(|e| format!("Could not create {}: {e}", dir.display()))?;
    for (index, data) in array.iter().enumerate() {
        let target = face_path(dir, index);
        let tmp = target.with_extension("png.tmp");
        fs::write(&tmp, data).map_err(|e| format!("Could not save {}: {e}", tmp.display()))?;
        fs::rename(&tmp, &target)
            .map_err(|e| format!("Could not save {}: {e}", target.display()))?;
    }
    Ok(dims)
}

/// Remove all imported panorama faces. Missing files are not an error.
pub fn clear_custom_panorama(dir: &Path) -> Result<(), String> {
    for i in 0..4 {
        let path = face_path(dir, i);
        match fs::remove_file(&path) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(format!("Could not remove {}: {e}", path.display())),
        }
    }
    Ok(())
}

/// Read the imported panorama faces from `dir` and stitch them into a wide strip.
pub fn load_custom_panorama(dir: &Path, max_face_dim: u32) -> Result<DecodedImage, String> {
    let mut faces = Vec::with_capacity(4);
    for i in 0..4 {
        let path = face_path(dir, i);
        let bytes = read_face(&path)?;
        faces.push(decode_png(&bytes, max_face_dim)?);
    }
    stitch_faces(&faces)
}

fn stitch_faces(faces: &[DecodedImage]) -> Result<DecodedImage, String> {
    if faces.len() != 4 {
        return Err("Exactly four panorama faces are required.".to_string());
    }
    let (fw, fh) = (faces[0].width, faces[0].height);
    if faces.iter().any(|f| f.width != fw || f.height != fh) {
        return Err("panorama faces have different sizes".to_string());
    }
    let mut rgba = vec![0u8; fw * 4 * fh * 4];
    for (index, face) in faces.iter().enumerate() {
        for y in 0..fh {
            let src = &face.rgba[y * fw * 4..(y + 1) * fw * 4];
            let dst_start = (y * fw * 4 + index * fw) * 4;
            rgba[dst_start..dst_start + fw * 4].copy_from_slice(src);
        }
    }
    Ok(DecodedImage {
        width: fw * 4,
        height: fh,
        rgba,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::resource_pack::tests::make_zip;
    use crate::test_support::{TempDir, encode_png};

    fn solid(c: u8) -> Vec<u8> {
        encode_png(64, 64, &[c, 0, 0, 255].repeat(64 * 64))
    }

    #[test]
    fn finds_the_client_jar() {
        let t = TempDir::new();
        assert!(find_client_jar(t.path(), "1.21.11").is_none());
        let dir = t.path().join("versions").join("1.21.11");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("1.21.11.jar"), "x").unwrap();
        assert!(find_client_jar(t.path(), "1.21.11").is_some());
    }

    #[test]
    fn stitches_four_faces_left_to_right() {
        let t = TempDir::new();
        let jar = t.path().join("client.jar");
        let faces: Vec<(String, Vec<u8>)> = (0..4)
            .map(|i| {
                (
                    format!("assets/minecraft/textures/gui/title/background/panorama_{i}.png"),
                    solid(10 * (i as u8 + 1)),
                )
            })
            .collect();
        let entries: Vec<(&str, &[u8])> = faces
            .iter()
            .map(|(n, d)| (n.as_str(), d.as_slice()))
            .collect();
        make_zip(&jar, &entries);

        let img = load_panorama(&jar, 512).unwrap();
        assert_eq!((img.width, img.height), (256, 64));
        let reds: Vec<u8> = (0..4).map(|f| img.rgba[f * 64 * 4]).collect();
        assert_eq!(reds, [10, 20, 30, 40]);
    }

    #[test]
    fn missing_faces_are_an_error() {
        let t = TempDir::new();
        let jar = t.path().join("client.jar");
        make_zip(&jar, &[("other.txt", b"x")]);
        assert!(load_panorama(&jar, 512).unwrap_err().contains("not found"));
        let bad = t.path().join("bad.jar");
        std::fs::write(&bad, "not a zip").unwrap();
        assert!(load_panorama(&bad, 512).is_err());
    }

    #[test]
    fn validates_compatible_face_dimensions() {
        let faces = [solid(1), solid(2), solid(3), solid(4)];
        assert_eq!(validate_face_bytes(&faces).unwrap(), (64, 64));

        let mut mismatched = faces.clone();
        mismatched[2] = encode_png(128, 128, &[0u8; 128 * 128 * 4]);
        let err = validate_face_bytes(&mismatched).unwrap_err();
        assert!(err.contains("same size"), "{err}");

        let mut non_square = faces.clone();
        non_square[0] = encode_png(64, 32, &[0u8; 64 * 32 * 4]);
        assert!(
            validate_face_bytes(&non_square)
                .unwrap_err()
                .contains("square")
        );
    }

    #[test]
    fn imports_and_restores_custom_faces() {
        let t = TempDir::new();
        let sources: Vec<PathBuf> = (0..4)
            .map(|i| {
                let p = t.path().join(format!("face{i}.png"));
                std::fs::write(&p, solid(20 * (i + 1))).unwrap();
                p
            })
            .collect();
        let source_array: [PathBuf; 4] = sources.clone().try_into().unwrap();
        let target = t.path().join("custom").join("panorama");
        assert_eq!(
            import_custom_panorama(&source_array, &target).unwrap(),
            (64, 64)
        );
        let stitched = load_custom_panorama(&target, 512).unwrap();
        assert_eq!((stitched.width, stitched.height), (256, 64));
        let reds: Vec<u8> = (0..4).map(|f| stitched.rgba[f * 64 * 4]).collect();
        assert_eq!(reds, [20, 40, 60, 80]);

        std::fs::write(&sources[0], encode_png(32, 32, &[0u8; 32 * 32 * 4])).unwrap();
        assert!(import_custom_panorama(&source_array, &target).is_err());
        assert!(load_custom_panorama(&target, 512).is_ok());
    }
}
