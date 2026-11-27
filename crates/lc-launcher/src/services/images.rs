//! PNG decoding and an asynchronous texture cache.

use std::collections::HashMap;
use std::fs;
use std::io::Cursor;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use egui::{ColorImage, Context, TextureHandle, TextureOptions};

use super::panorama;

const MAX_PIXELS: u64 = 64 * 1024 * 1024;
const MAX_FILE_BYTES: u64 = 64 * 1024 * 1024;

/// A decoded RGBA8 image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedImage {
    /// Width in pixels.
    pub width: usize,
    /// Height in pixels.
    pub height: usize,
    /// `width * height * 4` bytes of unmultiplied RGBA.
    pub rgba: Vec<u8>,
}

impl DecodedImage {
    fn into_color_image(self) -> ColorImage {
        ColorImage::from_rgba_unmultiplied([self.width, self.height], &self.rgba)
    }
}

/// Read the pixel dimensions of a PNG without decoding its pixels.
pub fn png_dimensions(bytes: &[u8]) -> Result<(u32, u32), String> {
    let decoder = png::Decoder::new(Cursor::new(bytes));
    let reader = decoder.read_info().map_err(|e| format!("PNG error: {e}"))?;
    let info = reader.info();
    if info.width == 0 || info.height == 0 {
        return Err("the image has no pixels".to_string());
    }
    Ok((info.width, info.height))
}

/// Decode PNG bytes to RGBA8, shrinking so the longest side is at most `max_dim`.
pub fn decode_png(bytes: &[u8], max_dim: u32) -> Result<DecodedImage, String> {
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().map_err(|e| format!("PNG error: {e}"))?;
    let (w, h) = {
        let info = reader.info();
        (info.width, info.height)
    };
    if w == 0 || h == 0 || u64::from(w) * u64::from(h) > MAX_PIXELS {
        return Err(format!("image size {w}x{h} is not supported"));
    }
    let size = reader
        .output_buffer_size()
        .ok_or_else(|| "image is too large".to_string())?;
    let mut buf = vec![0u8; size];
    let frame = reader
        .next_frame(&mut buf)
        .map_err(|e| format!("PNG error: {e}"))?;
    let (w, h) = (frame.width as usize, frame.height as usize);
    let data = &buf[..frame.buffer_size()];

    let mut rgba = Vec::with_capacity(w * h * 4);
    match frame.color_type {
        png::ColorType::Rgba => rgba.extend_from_slice(data),
        png::ColorType::Rgb => {
            for px in data.chunks_exact(3) {
                rgba.extend_from_slice(&[px[0], px[1], px[2], 255]);
            }
        }
        png::ColorType::Grayscale => {
            for &g in data {
                rgba.extend_from_slice(&[g, g, g, 255]);
            }
        }
        png::ColorType::GrayscaleAlpha => {
            for px in data.chunks_exact(2) {
                rgba.extend_from_slice(&[px[0], px[0], px[0], px[1]]);
            }
        }
        png::ColorType::Indexed => return Err("unexpanded palette image".to_string()),
    }
    if rgba.len() != w * h * 4 {
        return Err("unexpected PNG data length".to_string());
    }
    Ok(downscale(
        DecodedImage {
            width: w,
            height: h,
            rgba,
        },
        max_dim,
    ))
}

/// Box-filter shrink so the longest side is at most `max_dim`.
pub fn downscale(img: DecodedImage, max_dim: u32) -> DecodedImage {
    let max_dim = max_dim.max(1) as usize;
    let longest = img.width.max(img.height);
    if longest <= max_dim {
        return img;
    }
    let nw = (img.width * max_dim / longest).max(1);
    let nh = (img.height * max_dim / longest).max(1);
    let mut out = Vec::with_capacity(nw * nh * 4);
    for y in 0..nh {
        let y0 = y * img.height / nh;
        let y1 = ((y + 1) * img.height / nh).max(y0 + 1).min(img.height);
        for x in 0..nw {
            let x0 = x * img.width / nw;
            let x1 = ((x + 1) * img.width / nw).max(x0 + 1).min(img.width);
            let mut sum = [0u32; 4];
            for sy in y0..y1 {
                for sx in x0..x1 {
                    let i = (sy * img.width + sx) * 4;
                    for (c, s) in sum.iter_mut().enumerate() {
                        *s += u32::from(img.rgba[i + c]);
                    }
                }
            }
            let n = ((y1 - y0) * (x1 - x0)) as u32;
            out.extend(sum.iter().map(|s| (s / n) as u8));
        }
    }
    DecodedImage {
        width: nw,
        height: nh,
        rgba: out,
    }
}

/// Where an image comes from.
#[derive(Debug, Clone)]
pub enum ImageSource {
    /// A PNG file on disk.
    PngFile(PathBuf),
    /// The stitched vanilla panorama inside a Minecraft client jar.
    Panorama(PathBuf),
    /// Four imported images stitched into a panorama.
    CustomPanorama(PathBuf),
}

struct Job {
    key: String,
    source: ImageSource,
    max_dim: u32,
}

type JobResult = (String, Result<DecodedImage, String>);

fn run_job(job: &Job) -> Result<DecodedImage, String> {
    match &job.source {
        ImageSource::PngFile(path) => {
            let meta = fs::metadata(path).map_err(|e| e.to_string())?;
            if meta.len() > MAX_FILE_BYTES {
                return Err("file is too large".to_string());
            }
            let bytes = fs::read(path).map_err(|e| e.to_string())?;
            decode_png(&bytes, job.max_dim)
        }
        ImageSource::Panorama(jar) => panorama::load_panorama(jar, job.max_dim),
        ImageSource::CustomPanorama(dir) => panorama::load_custom_panorama(dir, job.max_dim),
    }
}

enum Entry {
    Loading,
    Ready(TextureHandle),
    Failed,
}

/// Loads images on a worker thread and caches them as GPU textures.
pub struct TextureCache {
    ctx: Context,
    entries: HashMap<String, Entry>,
    jobs: Sender<Job>,
    results: Receiver<JobResult>,
}

impl TextureCache {
    /// Create the cache and start its worker thread.
    pub fn new(ctx: &Context) -> Self {
        let (job_tx, job_rx) = mpsc::channel::<Job>();
        let (res_tx, res_rx) = mpsc::channel::<JobResult>();
        let repaint = ctx.clone();
        let _ = thread::Builder::new()
            .name("lc-images".into())
            .spawn(move || {
                while let Ok(job) = job_rx.recv() {
                    let result = run_job(&job);
                    if res_tx.send((job.key, result)).is_err() {
                        break;
                    }
                    repaint.request_repaint();
                }
            });
        Self {
            ctx: ctx.clone(),
            entries: HashMap::new(),
            jobs: job_tx,
            results: res_rx,
        }
    }

    /// Start loading `key` from `source` unless it is already loading or loaded.
    pub fn request(&mut self, key: &str, source: ImageSource, max_dim: u32) {
        if self.entries.contains_key(key) {
            return;
        }
        let sent = self.jobs.send(Job {
            key: key.to_string(),
            source,
            max_dim,
        });
        self.entries.insert(
            key.to_string(),
            if sent.is_ok() {
                Entry::Loading
            } else {
                Entry::Failed
            },
        );
    }

    /// Decode PNG `bytes` right now (for small images such as icons) and cache the texture.
    pub fn load_bytes(&mut self, key: &str, bytes: &[u8], max_dim: u32) {
        self.load_bytes_with(key, bytes, max_dim, TextureOptions::LINEAR);
    }

    /// Like [`Self::load_bytes`] but keeps hard pixel edges, for Minecraft textures.
    pub fn load_bytes_nearest(&mut self, key: &str, bytes: &[u8], max_dim: u32) {
        self.load_bytes_with(key, bytes, max_dim, TextureOptions::NEAREST);
    }

    fn load_bytes_with(&mut self, key: &str, bytes: &[u8], max_dim: u32, options: TextureOptions) {
        if self.entries.contains_key(key) {
            return;
        }
        let entry = match decode_png(bytes, max_dim) {
            Ok(img) => Entry::Ready(self.ctx.load_texture(key, img.into_color_image(), options)),
            Err(_) => Entry::Failed,
        };
        self.entries.insert(key.to_string(), entry);
    }

    /// Upload finished images. Returns whether anything changed.
    pub fn poll(&mut self) -> bool {
        let mut changed = false;
        while let Ok((key, result)) = self.results.try_recv() {
            let entry = match result {
                Ok(img) => Entry::Ready(self.ctx.load_texture(
                    &key,
                    img.into_color_image(),
                    TextureOptions::LINEAR,
                )),
                Err(_) => Entry::Failed,
            };
            self.entries.insert(key, entry);
            changed = true;
        }
        changed
    }

    /// The texture for `key`, once loaded.
    pub fn get(&self, key: &str) -> Option<&TextureHandle> {
        match self.entries.get(key) {
            Some(Entry::Ready(t)) => Some(t),
            _ => None,
        }
    }

    /// Whether `key` is still loading.
    #[cfg(test)]
    pub fn is_loading(&self, key: &str) -> bool {
        matches!(self.entries.get(key), Some(Entry::Loading))
    }

    /// Whether `key` failed to load.
    pub fn has_failed(&self, key: &str) -> bool {
        matches!(self.entries.get(key), Some(Entry::Failed))
    }

    /// Drop a cached image so it can be loaded again.
    pub fn evict(&mut self, key: &str) {
        self.entries.remove(key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::encode_png;

    fn gradient(w: usize, h: usize) -> Vec<u8> {
        let mut v = Vec::new();
        for y in 0..h {
            for x in 0..w {
                v.extend_from_slice(&[(x * 255 / w) as u8, (y * 255 / h) as u8, 7, 255]);
            }
        }
        v
    }

    #[test]
    fn decodes_rgba_png() {
        let png = encode_png(4, 3, &gradient(4, 3));
        let img = decode_png(&png, 100).unwrap();
        assert_eq!((img.width, img.height), (4, 3));
        assert_eq!(img.rgba.len(), 4 * 3 * 4);
        assert_eq!(img.rgba[2], 7);
    }

    #[test]
    fn downscales_to_fit_keeping_aspect() {
        let png = encode_png(64, 32, &gradient(64, 32));
        let img = decode_png(&png, 16).unwrap();
        assert_eq!((img.width, img.height), (16, 8));
        assert_eq!(img.rgba.len(), 16 * 8 * 4);
    }

    #[test]
    fn downscale_averages_pixels() {
        let img = DecodedImage {
            width: 2,
            height: 1,
            rgba: vec![0, 0, 0, 255, 100, 100, 100, 255],
        };
        let out = downscale(img, 1);
        assert_eq!((out.width, out.height), (1, 1));
        assert_eq!(out.rgba, vec![50, 50, 50, 255]);
    }

    #[test]
    fn rejects_garbage() {
        assert!(decode_png(b"not a png", 100).is_err());
        assert!(decode_png(&[], 100).is_err());
    }

    #[test]
    fn texture_cache_loads_in_background() {
        let ctx = Context::default();
        let dir = crate::test_support::TempDir::new();
        let path = dir.path().join("a.png");
        fs::write(&path, encode_png(8, 8, &gradient(8, 8))).unwrap();
        fs::write(dir.path().join("bad.png"), "nope").unwrap();

        let mut cache = TextureCache::new(&ctx);
        cache.request("ok", ImageSource::PngFile(path.clone()), 64);
        cache.request("bad", ImageSource::PngFile(dir.path().join("bad.png")), 64);
        cache.request("ok", ImageSource::PngFile(path), 64);
        assert!(cache.is_loading("ok"));

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while (cache.is_loading("ok") || cache.is_loading("bad"))
            && std::time::Instant::now() < deadline
        {
            cache.poll();
            thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(cache.get("ok").is_some());
        assert!(cache.has_failed("bad"));
        cache.evict("bad");
        assert!(!cache.has_failed("bad"));
    }

    #[test]
    fn bytes_load_synchronously() {
        let ctx = Context::default();
        let mut cache = TextureCache::new(&ctx);
        cache.load_bytes("icon", &encode_png(2, 2, &gradient(2, 2)), 64);
        assert!(cache.get("icon").is_some());
        cache.load_bytes("broken", b"xx", 64);
        assert!(cache.has_failed("broken"));
    }
}
