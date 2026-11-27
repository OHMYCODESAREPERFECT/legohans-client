//! Embed the launcher icon into Windows executables.
//!
//! The pixels come from `lc-launcher/src/icon.rs`, so the window icon and the executable icon
//! can never drift apart. On other platforms the build script is a no-op.

#[cfg(windows)]
#[path = "../lc-launcher/src/icon.rs"]
mod icon;

#[cfg(windows)]
fn write_ico(path: &std::path::Path, rgba: &[u8], size: u32) {
    let side = size as usize;
    let mut image = Vec::new();
    // BITMAPINFOHEADER; height doubles because an ICO stores an XOR bitmap and an AND mask.
    image.extend_from_slice(&40u32.to_le_bytes());
    image.extend_from_slice(&size.to_le_bytes());
    image.extend_from_slice(&(size * 2).to_le_bytes());
    image.extend_from_slice(&1u16.to_le_bytes());
    image.extend_from_slice(&32u16.to_le_bytes());
    image.extend_from_slice(&0u32.to_le_bytes()); // BI_RGB
    image.extend_from_slice(&((side * side * 4) as u32).to_le_bytes());
    image.extend_from_slice(&[0u8; 16]); // resolution and palette fields

    // DIB rows are bottom-up and use BGRA order.
    for y in (0..side).rev() {
        for x in 0..side {
            let i = (y * side + x) * 4;
            image.extend_from_slice(&[rgba[i + 2], rgba[i + 1], rgba[i], rgba[i + 3]]);
        }
    }
    // The alpha channel carries transparency, but the legacy 1-bit mask still needs the correct
    // bits for monochrome rendering and drag silhouettes.
    image.extend_from_slice(&icon::ico_and_mask(rgba, side));

    let mut ico = Vec::with_capacity(22 + image.len());
    ico.extend_from_slice(&[
        0, 0, // reserved
        1, 0, // type: icon
        1, 0, // one image
    ]);
    ico.push(size as u8);
    ico.push(size as u8);
    ico.extend_from_slice(&[
        0, // no palette
        0, // reserved
        1, 0, // colour planes
        32, 0, // bits per pixel
    ]);
    ico.extend_from_slice(&(image.len() as u32).to_le_bytes());
    ico.extend_from_slice(&22u32.to_le_bytes());
    ico.extend_from_slice(&image);
    std::fs::write(path, ico).expect("write the generated icon");
}

fn main() {
    println!("cargo:rerun-if-changed=../lc-launcher/src/icon.rs");
    #[cfg(windows)]
    {
        let out = std::env::var_os("OUT_DIR").expect("Cargo sets OUT_DIR");
        let ico = std::path::Path::new(&out).join("lc-app.ico");
        write_ico(&ico, &icon::rgba(), icon::SIZE as u32);
        winres::WindowsResource::new()
            .set_icon(ico.to_str().expect("a UTF-8 OUT_DIR path"))
            .compile()
            .expect("embed the Windows icon resource");
        // The GNU linker drops the resource static library because nothing references a symbol
        // in it. Passing the COFF object directly forces the resource into both binaries.
        if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("gnu") {
            println!(
                "cargo:rustc-link-arg={}",
                std::path::Path::new(&out).join("resource.o").display()
            );
        }
    }
}
