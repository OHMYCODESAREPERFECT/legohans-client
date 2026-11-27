//! Loading the vanilla title-screen panorama from the user's own Minecraft install.
//!
//! The launcher never ships Mojang artwork. If the official client jar is present on the
//! user's machine, the four horizontal panorama faces are read from it at runtime.

use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use super::images::{DecodedImage, decode_png};

const FACE_LIMIT: u64 = 8 * 1024 * 1024;

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
        encode_png(2, 2, &[c, 0, 0, 255].repeat(4))
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
        assert_eq!((img.width, img.height), (8, 2));
        // Red channel of the first pixel of each 2px-wide face.
        let reds: Vec<u8> = (0..4).map(|f| img.rgba[f * 2 * 4]).collect();
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
}
