//! Bundled Java Edition UI artwork and nearest-filtered texture handles.
//!
//! The visual-only menu uses the original files supplied for this milestone. Large logos are
//! box-downscaled once at load time so the biggest GPU texture side stays small, while every
//! widget keeps hard pixel edges.

use std::io::Cursor;

use egui::{ColorImage, Context, TextureHandle, TextureOptions};

/// A decoded RGBA8 image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedImage {
    /// Image width in pixels.
    pub width: usize,
    /// Image height in pixels.
    pub height: usize,
    /// Unmultiplied RGBA bytes.
    pub rgba: Vec<u8>,
}

/// Decode a bundled PNG into RGBA8.
pub fn decode(bytes: &[u8]) -> Result<DecodedImage, String> {
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().map_err(|e| format!("PNG error: {e}"))?;
    let mut buf = vec![
        0u8;
        reader
            .output_buffer_size()
            .ok_or_else(|| "image is too large".to_string())?
    ];
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
    Ok(DecodedImage {
        width: w,
        height: h,
        rgba,
    })
}

/// Box-filter an image so its longest side is at most `max_side`.
pub fn downscale(image: DecodedImage, max_side: usize) -> DecodedImage {
    let longest = image.width.max(image.height);
    if longest <= max_side {
        return image;
    }
    let factor = longest as f32 / max_side as f32;
    let new_w = (image.width as f32 / factor).round().max(1.0) as usize;
    let new_h = (image.height as f32 / factor).round().max(1.0) as usize;
    let mut rgba = vec![0u8; new_w * new_h * 4];
    for y in 0..new_h {
        let y0 = (y * image.height / new_h).min(image.height - 1);
        let y1 = ((y + 1) * image.height / new_h)
            .max(y0 + 1)
            .min(image.height);
        for x in 0..new_w {
            let x0 = (x * image.width / new_w).min(image.width - 1);
            let x1 = ((x + 1) * image.width / new_w).max(x0 + 1).min(image.width);
            let mut sum = [0u32; 4];
            let mut count = 0u32;
            for sy in y0..y1 {
                for sx in x0..x1 {
                    let p = (sy * image.width + sx) * 4;
                    for (c, total) in sum.iter_mut().enumerate() {
                        *total += u32::from(image.rgba[p + c]);
                    }
                    count += 1;
                }
            }
            let out = (y * new_w + x) * 4;
            for (c, total) in sum.iter().enumerate() {
                rgba[out + c] = (*total / count) as u8;
            }
        }
    }
    DecodedImage {
        width: new_w,
        height: new_h,
        rgba,
    }
}

fn handle(ctx: &Context, name: &str, bytes: &[u8], max_side: usize) -> TextureHandle {
    let image = downscale(
        decode(bytes).unwrap_or_else(|e| panic!("invalid bundled {name}: {e}")),
        max_side,
    );
    ctx.load_texture(
        name,
        ColorImage::from_rgba_unmultiplied([image.width, image.height], &image.rgba),
        TextureOptions::NEAREST,
    )
}

/// Panorama face bytes, in Java Edition file order.
pub static PANORAMA: [&[u8]; 6] = [
    include_bytes!("../../assets/ingame/title/background/panorama_0.png"),
    include_bytes!("../../assets/ingame/title/background/panorama_1.png"),
    include_bytes!("../../assets/ingame/title/background/panorama_2.png"),
    include_bytes!("../../assets/ingame/title/background/panorama_3.png"),
    include_bytes!("../../assets/ingame/title/background/panorama_4.png"),
    include_bytes!("../../assets/ingame/title/background/panorama_5.png"),
];

/// Soft overlay applied over the panorama.
pub static PANORAMA_OVERLAY: &[u8] =
    include_bytes!("../../assets/ingame/title/background/panorama_overlay.png");
/// Static panorama face used as the menu backdrop instead of an animated cube.
pub static PANORAMA_STILL: &[u8] =
    include_bytes!("../../assets/ingame/title/background/panorama_0.png");

/// Official 1024x176 Minecraft wordmark.
pub static LOGO: &[u8] = include_bytes!("../../assets/ingame/title/minecraft.png");
/// Alternative wordmark, retained for comparison in asset tests.
pub static CREEPER_LOGO: &[u8] = include_bytes!("../../assets/ingame/title/minceraft.png");
/// Java Edition lockup placed under the wordmark.
pub static EDITION: &[u8] = include_bytes!("../../assets/ingame/title/edition.png");
/// Realms wordmark used by the multiplayer header.
pub static REALMS: &[u8] = include_bytes!("../../assets/ingame/title/realms.png");
/// Mojang Studios logo used by the loading screen.
pub static MOJANG: &[u8] = include_bytes!("../../assets/ingame/title/mojangstudios.png");
/// Full Mojang Studios splash reference used by the loading screen.
pub static LOADING: &[u8] = include_bytes!("../../assets/ingame/title/loading.png");
/// Minecraft bitmap font atlas.
pub static FONT: &[u8] = include_bytes!("../../assets/ingame/font/ascii.png");

/// Standard dark menu panel.
pub static MENU_BACKGROUND: &[u8] = include_bytes!("../../assets/ingame/gui/menu_background.png");
/// Standard dark list panel.
pub static MENU_LIST_BACKGROUND: &[u8] =
    include_bytes!("../../assets/ingame/gui/menu_list_background.png");
/// Pause-menu panel.
pub static INWORLD_BACKGROUND: &[u8] =
    include_bytes!("../../assets/ingame/gui/inworld_menu_background.png");
/// Pause-menu list panel.
pub static INWORLD_LIST_BACKGROUND: &[u8] =
    include_bytes!("../../assets/ingame/gui/inworld_menu_list_background.png");
/// Light top separator.
pub static HEADER_SEPARATOR: &[u8] = include_bytes!("../../assets/ingame/gui/header_separator.png");
/// Light bottom separator.
pub static FOOTER_SEPARATOR: &[u8] = include_bytes!("../../assets/ingame/gui/footer_separator.png");
/// Pause-menu top separator.
pub static INWORLD_HEADER_SEPARATOR: &[u8] =
    include_bytes!("../../assets/ingame/gui/inworld_header_separator.png");
/// Pause-menu bottom separator.
pub static INWORLD_FOOTER_SEPARATOR: &[u8] =
    include_bytes!("../../assets/ingame/gui/inworld_footer_separator.png");

/// Normal button nine-slice.
pub static BUTTON: &[u8] = include_bytes!("../../assets/ingame/widget/button.png");
/// Hovered button nine-slice.
pub static BUTTON_HIGHLIGHTED: &[u8] =
    include_bytes!("../../assets/ingame/widget/button_highlighted.png");
/// Disabled button nine-slice.
pub static BUTTON_DISABLED: &[u8] =
    include_bytes!("../../assets/ingame/widget/button_disabled.png");
/// Text field nine-slice.
pub static TEXT_FIELD: &[u8] = include_bytes!("../../assets/ingame/widget/text_field.png");
/// Focused text field nine-slice.
pub static TEXT_FIELD_HIGHLIGHTED: &[u8] =
    include_bytes!("../../assets/ingame/widget/text_field_highlighted.png");
/// Slider track nine-slice.
pub static SLIDER: &[u8] = include_bytes!("../../assets/ingame/widget/slider.png");
/// Hovered slider track nine-slice.
pub static SLIDER_HIGHLIGHTED: &[u8] =
    include_bytes!("../../assets/ingame/widget/slider_highlighted.png");
/// Slider handle.
pub static SLIDER_HANDLE: &[u8] = include_bytes!("../../assets/ingame/widget/slider_handle.png");
/// Hovered slider handle.
pub static SLIDER_HANDLE_HIGHLIGHTED: &[u8] =
    include_bytes!("../../assets/ingame/widget/slider_handle_highlighted.png");
/// Empty checkbox.
pub static CHECKBOX: &[u8] = include_bytes!("../../assets/ingame/widget/checkbox.png");
/// Hovered empty checkbox.
pub static CHECKBOX_HIGHLIGHTED: &[u8] =
    include_bytes!("../../assets/ingame/widget/checkbox_highlighted.png");
/// Selected checkbox.
pub static CHECKBOX_SELECTED: &[u8] =
    include_bytes!("../../assets/ingame/widget/checkbox_selected.png");
/// Hovered selected checkbox.
pub static CHECKBOX_SELECTED_HIGHLIGHTED: &[u8] =
    include_bytes!("../../assets/ingame/widget/checkbox_selected_highlighted.png");
/// Scrollbar thumb.
pub static SCROLLER: &[u8] = include_bytes!("../../assets/ingame/widget/scroller.png");
/// Scrollbar track.
pub static SCROLLER_BACKGROUND: &[u8] =
    include_bytes!("../../assets/ingame/widget/scroller_background.png");
/// Small close button.
pub static CROSS: &[u8] = include_bytes!("../../assets/ingame/widget/cross_button.png");
/// Hovered small close button.
pub static CROSS_HIGHLIGHTED: &[u8] =
    include_bytes!("../../assets/ingame/widget/cross_button_highlighted.png");

/// Selected-world marker.
pub static WORLD_SELECTED: &[u8] = include_bytes!("../../assets/ingame/world_list/marked_join.png");
/// World icon.
pub static WORLD_JOIN: &[u8] = include_bytes!("../../assets/ingame/world_list/join.png");
/// World warning icon.
pub static WORLD_WARNING: &[u8] = include_bytes!("../../assets/ingame/world_list/warning.png");
/// Ping strength icons, weakest to strongest.
pub static PING: [&[u8]; 5] = [
    include_bytes!("../../assets/ingame/server_list/ping_1.png"),
    include_bytes!("../../assets/ingame/server_list/ping_2.png"),
    include_bytes!("../../assets/ingame/server_list/ping_3.png"),
    include_bytes!("../../assets/ingame/server_list/ping_4.png"),
    include_bytes!("../../assets/ingame/server_list/ping_5.png"),
];
/// Unreachable server icon.
pub static SERVER_UNREACHABLE: &[u8] =
    include_bytes!("../../assets/ingame/server_list/unreachable.png");

/// All cached texture handles used by the menus.
#[derive(Clone)]
pub struct Textures {
    /// Wordmark.
    pub logo: TextureHandle,
    /// Java Edition lockup.
    pub edition: TextureHandle,
    /// Optional Realms artwork.
    pub realms: TextureHandle,
    /// Mojang Studios logo.
    pub mojang: TextureHandle,
    /// Full Mojang Studios splash.
    pub loading: TextureHandle,
    /// Panorama vignette.
    pub overlay: TextureHandle,
    /// Static panorama face used behind menus.
    pub still_panorama: TextureHandle,
    /// Bitmap font atlas.
    pub font: TextureHandle,
    /// Dark menu panel.
    pub menu: TextureHandle,
    /// Dark list panel.
    pub list: TextureHandle,
    /// Pause menu panel.
    pub inworld_menu: TextureHandle,
    /// Pause menu list.
    pub inworld_list: TextureHandle,
    /// Top separator.
    pub header: TextureHandle,
    /// Bottom separator.
    pub footer: TextureHandle,
    /// Pause top separator.
    pub inworld_header: TextureHandle,
    /// Pause bottom separator.
    pub inworld_footer: TextureHandle,
    /// Normal button.
    pub button: TextureHandle,
    /// Hovered button.
    pub button_hover: TextureHandle,
    /// Disabled button.
    pub button_disabled: TextureHandle,
    /// Text field.
    pub text_field: TextureHandle,
    /// Focused text field.
    pub text_field_focused: TextureHandle,
    /// Slider track.
    pub slider: TextureHandle,
    /// Hovered slider track.
    pub slider_hover: TextureHandle,
    /// Slider handle.
    pub slider_handle: TextureHandle,
    /// Hovered slider handle.
    pub slider_handle_hover: TextureHandle,
    /// Empty checkbox.
    pub checkbox: TextureHandle,
    /// Hovered empty checkbox.
    pub checkbox_hover: TextureHandle,
    /// Selected checkbox.
    pub checkbox_selected: TextureHandle,
    /// Hovered selected checkbox.
    pub checkbox_selected_hover: TextureHandle,
    /// Scrollbar thumb.
    pub scroller: TextureHandle,
    /// Scrollbar track.
    pub scroller_bg: TextureHandle,
    /// Close button.
    pub cross: TextureHandle,
    /// Hovered close button.
    pub cross_hover: TextureHandle,
    /// World marker.
    pub world_selected: TextureHandle,
    /// World icon.
    pub world_join: TextureHandle,
    /// World warning.
    pub world_warning: TextureHandle,
    /// Ping icons.
    pub ping: [TextureHandle; 5],
    /// Unreachable server icon.
    pub server_unreachable: TextureHandle,
}

impl Textures {
    /// Load every bundled texture with nearest-neighbor sampling.
    pub fn load(ctx: &Context) -> Self {
        Self {
            logo: handle(ctx, "ingame/logo", LOGO, 512),
            edition: handle(ctx, "ingame/edition", EDITION, 512),
            realms: handle(ctx, "ingame/realms", REALMS, 512),
            mojang: handle(ctx, "ingame/mojang", MOJANG, 512),
            loading: handle(ctx, "ingame/loading", LOADING, 720),
            overlay: handle(ctx, "ingame/overlay", PANORAMA_OVERLAY, 512),
            still_panorama: handle(ctx, "ingame/still-panorama", PANORAMA_STILL, 1024),
            font: handle(ctx, "ingame/font", FONT, 128),
            menu: handle(ctx, "ingame/menu", MENU_BACKGROUND, 16),
            list: handle(ctx, "ingame/list", MENU_LIST_BACKGROUND, 16),
            inworld_menu: handle(ctx, "ingame/inworld-menu", INWORLD_BACKGROUND, 16),
            inworld_list: handle(ctx, "ingame/inworld-list", INWORLD_LIST_BACKGROUND, 16),
            header: handle(ctx, "ingame/header", HEADER_SEPARATOR, 32),
            footer: handle(ctx, "ingame/footer", FOOTER_SEPARATOR, 32),
            inworld_header: handle(ctx, "ingame/inworld-header", INWORLD_HEADER_SEPARATOR, 32),
            inworld_footer: handle(ctx, "ingame/inworld-footer", INWORLD_FOOTER_SEPARATOR, 32),
            button: handle(ctx, "ingame/button", BUTTON, 200),
            button_hover: handle(ctx, "ingame/button-hover", BUTTON_HIGHLIGHTED, 200),
            button_disabled: handle(ctx, "ingame/button-disabled", BUTTON_DISABLED, 200),
            text_field: handle(ctx, "ingame/text-field", TEXT_FIELD, 200),
            text_field_focused: handle(
                ctx,
                "ingame/text-field-focused",
                TEXT_FIELD_HIGHLIGHTED,
                200,
            ),
            slider: handle(ctx, "ingame/slider", SLIDER, 200),
            slider_hover: handle(ctx, "ingame/slider-hover", SLIDER_HIGHLIGHTED, 200),
            slider_handle: handle(ctx, "ingame/slider-handle", SLIDER_HANDLE, 8),
            slider_handle_hover: handle(
                ctx,
                "ingame/slider-handle-hover",
                SLIDER_HANDLE_HIGHLIGHTED,
                8,
            ),
            checkbox: handle(ctx, "ingame/checkbox", CHECKBOX, 20),
            checkbox_hover: handle(ctx, "ingame/checkbox-hover", CHECKBOX_HIGHLIGHTED, 20),
            checkbox_selected: handle(ctx, "ingame/checkbox-selected", CHECKBOX_SELECTED, 20),
            checkbox_selected_hover: handle(
                ctx,
                "ingame/checkbox-selected-hover",
                CHECKBOX_SELECTED_HIGHLIGHTED,
                20,
            ),
            scroller: handle(ctx, "ingame/scroller", SCROLLER, 6),
            scroller_bg: handle(ctx, "ingame/scroller-bg", SCROLLER_BACKGROUND, 6),
            cross: handle(ctx, "ingame/cross", CROSS, 14),
            cross_hover: handle(ctx, "ingame/cross-hover", CROSS_HIGHLIGHTED, 14),
            world_selected: handle(ctx, "ingame/world-selected", WORLD_SELECTED, 32),
            world_join: handle(ctx, "ingame/world-join", WORLD_JOIN, 32),
            world_warning: handle(ctx, "ingame/world-warning", WORLD_WARNING, 32),
            ping: [
                handle(ctx, "ingame/ping-1", PING[0], 8),
                handle(ctx, "ingame/ping-2", PING[1], 8),
                handle(ctx, "ingame/ping-3", PING[2], 8),
                handle(ctx, "ingame/ping-4", PING[3], 8),
                handle(ctx, "ingame/ping-5", PING[4], 8),
            ],
            server_unreachable: handle(ctx, "ingame/server-unreachable", SERVER_UNREACHABLE, 8),
        }
    }
}
