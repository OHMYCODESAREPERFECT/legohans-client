//! Minecraft-style widgets drawn from the supplied nine-slice sprites.

use egui::{
    Color32, Context, Painter, Pos2, Rect, Response, Sense, TextureHandle, TextureId, Ui, Vec2,
    pos2, vec2,
};

use super::{assets::Textures, font::BitmapFont};

/// Shared visual context for every menu screen.
#[derive(Clone)]
pub struct Visuals {
    /// Integer GUI scale.
    pub scale: u32,
    /// Bitmap font.
    pub font: BitmapFont,
    /// Bundled textures.
    pub textures: Textures,
}

impl Visuals {
    /// Build the context for a frame.
    pub fn new(ctx: &Context) -> Self {
        let textures = Textures::load(ctx);
        let font = BitmapFont::new(ctx, textures.font.clone());
        Self {
            scale: 2,
            font,
            textures,
        }
    }

    /// Multiply a GUI-pixel length by the current scale.
    pub fn px(&self, value: f32) -> f32 {
        value * self.scale as f32
    }
}

fn uv_rect(texture: &TextureHandle, x: f32, y: f32, w: f32, h: f32) -> Rect {
    let size = texture.size_vec2();
    Rect::from_min_max(
        pos2(x / size.x, y / size.y),
        pos2((x + w) / size.x, (y + h) / size.y),
    )
}

/// Draw a 3x3 nine-slice from a bordered widget texture.
pub fn nine_slice(
    painter: &Painter,
    texture: &TextureHandle,
    rect: Rect,
    source_border: f32,
    unit: f32,
    tint: Color32,
) {
    let size = texture.size_vec2();
    let border = source_border * unit;
    let bw = rect.width().min(border * 2.0);
    let bh = rect.height().min(border * 2.0);
    let sw = size.x - source_border * 2.0;
    let sh = size.y - source_border * 2.0;
    let pieces = [
        (
            Rect::from_min_max(rect.min, rect.min + vec2(bw, bh)),
            uv_rect(texture, 0.0, 0.0, source_border, source_border),
        ),
        (
            Rect::from_min_max(
                pos2(rect.min.x + bw, rect.min.y),
                pos2(rect.max.x - bw, rect.min.y + bh),
            ),
            uv_rect(texture, source_border, 0.0, sw, source_border),
        ),
        (
            Rect::from_min_max(
                pos2(rect.max.x - bw, rect.min.y),
                pos2(rect.max.x, rect.min.y + bh),
            ),
            uv_rect(
                texture,
                size.x - source_border,
                0.0,
                source_border,
                source_border,
            ),
        ),
        (
            Rect::from_min_max(
                pos2(rect.min.x, rect.min.y + bh),
                pos2(rect.min.x + bw, rect.max.y - bh),
            ),
            uv_rect(texture, 0.0, source_border, source_border, sh),
        ),
        (
            Rect::from_min_max(
                pos2(rect.min.x + bw, rect.min.y + bh),
                pos2(rect.max.x - bw, rect.max.y - bh),
            ),
            uv_rect(texture, source_border, source_border, sw, sh),
        ),
        (
            Rect::from_min_max(
                pos2(rect.max.x - bw, rect.min.y + bh),
                pos2(rect.max.x, rect.max.y - bh),
            ),
            uv_rect(
                texture,
                size.x - source_border,
                source_border,
                source_border,
                sh,
            ),
        ),
        (
            Rect::from_min_max(
                pos2(rect.min.x, rect.max.y - bh),
                pos2(rect.min.x + bw, rect.max.y),
            ),
            uv_rect(
                texture,
                0.0,
                size.y - source_border,
                source_border,
                source_border,
            ),
        ),
        (
            Rect::from_min_max(
                pos2(rect.min.x + bw, rect.max.y - bh),
                pos2(rect.max.x - bw, rect.max.y),
            ),
            uv_rect(
                texture,
                source_border,
                size.y - source_border,
                sw,
                source_border,
            ),
        ),
        (
            Rect::from_min_max(pos2(rect.max.x - bw, rect.max.y - bh), rect.max),
            uv_rect(
                texture,
                size.x - source_border,
                size.y - source_border,
                source_border,
                source_border,
            ),
        ),
    ];
    for (dest, uv) in pieces {
        if dest.width() > 0.0 && dest.height() > 0.0 {
            painter.image(texture.id(), dest, uv, tint);
        }
    }
}

/// Stretch a small texture across a rectangle without changing its aspect-dependent corners.
pub fn fill(painter: &Painter, texture: &TextureHandle, rect: Rect, tint: Color32) {
    painter.image(
        texture.id(),
        rect,
        Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
        tint,
    );
}

/// A clickable Minecraft button. All buttons are visual-only; callers decide what a click previews.
pub fn button(ui: &mut Ui, rect: Rect, label: &str, visuals: &Visuals, enabled: bool) -> Response {
    let id = ui
        .id()
        .with(("mc-button", label, rect.min.x as i32, rect.min.y as i32));
    let response = ui.interact(
        rect,
        id,
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    if response.clicked() {
        response.request_focus();
    }
    let hovered = enabled && response.hovered();
    let pressed = enabled && response.is_pointer_button_down_on();
    let focused = enabled && response.has_focus();
    let texture = if !enabled {
        &visuals.textures.button_disabled
    } else if hovered {
        &visuals.textures.button_hover
    } else {
        &visuals.textures.button
    };
    let tint = if pressed {
        Color32::from_gray(170)
    } else {
        Color32::WHITE
    };
    nine_slice(ui.painter(), texture, rect, 3.0, visuals.scale as f32, tint);
    if focused {
        ui.painter().rect_stroke(
            rect.shrink(1.0),
            egui::CornerRadius::ZERO,
            (1.0, Color32::WHITE),
            egui::StrokeKind::Inside,
        );
    }
    let text_color = if enabled {
        Color32::from_rgb(255, 255, 255)
    } else {
        Color32::from_rgb(160, 160, 160)
    };
    let text_rect = if pressed {
        rect.translate(vec2(0.0, visuals.px(1.0)))
    } else {
        rect
    };
    visuals.font.draw_centered(
        ui.painter(),
        text_rect,
        label,
        visuals.scale,
        text_color,
        true,
    );
    response
}

/// Draw a static text line.
pub fn label(painter: &Painter, text: &str, pos: Pos2, visuals: &Visuals, color: Color32) {
    visuals
        .font
        .draw(painter, pos, text, visuals.scale, color, true);
}

/// Draw a centered title.
pub fn title(painter: &Painter, text: &str, rect: Rect, visuals: &Visuals, scale: u32) {
    visuals.font.draw_centered(
        painter,
        rect,
        text,
        scale,
        Color32::from_rgb(255, 255, 255),
        true,
    );
}

/// Draw a panel using the original dark menu texture.
pub fn panel(painter: &Painter, rect: Rect, visuals: &Visuals, inworld: bool) {
    let texture = if inworld {
        &visuals.textures.inworld_menu
    } else {
        &visuals.textures.menu
    };
    fill(painter, texture, rect, Color32::WHITE);
}

/// Draw a list background.
pub fn list_background(painter: &Painter, rect: Rect, visuals: &Visuals, inworld: bool) {
    let texture = if inworld {
        &visuals.textures.inworld_list
    } else {
        &visuals.textures.list
    };
    fill(painter, texture, rect, Color32::WHITE);
}

/// Draw a horizontal separator, optionally tiled to the full width.
pub fn separator(painter: &Painter, rect: Rect, visuals: &Visuals, inworld: bool, top: bool) {
    let texture = match (inworld, top) {
        (true, true) => &visuals.textures.inworld_header,
        (true, false) => &visuals.textures.inworld_footer,
        (false, true) => &visuals.textures.header,
        (false, false) => &visuals.textures.footer,
    };
    let height = visuals.px(2.0);
    let tile_w = visuals.px(32.0);
    let mut x = rect.min.x;
    while x < rect.max.x {
        let w = tile_w.min(rect.max.x - x);
        painter.image(
            texture.id(),
            Rect::from_min_max(pos2(x, rect.min.y), pos2(x + w, rect.min.y + height)),
            Rect::from_min_max(pos2(0.0, 0.0), pos2(w / tile_w, 1.0)),
            Color32::WHITE,
        );
        x += tile_w;
    }
}

/// Draw a checkbox and return its interaction.
pub fn checkbox(ui: &mut Ui, rect: Rect, checked: bool, visuals: &Visuals) -> Response {
    let response = ui.interact(
        rect,
        ui.id().with(("mc-check", rect.min.x as i32)),
        Sense::click(),
    );
    let hovered = response.hovered();
    let texture = match (checked, hovered) {
        (true, true) => &visuals.textures.checkbox_selected_hover,
        (true, false) => &visuals.textures.checkbox_selected,
        (false, true) => &visuals.textures.checkbox_hover,
        (false, false) => &visuals.textures.checkbox,
    };
    painter_image_scaled(ui.painter(), texture.id(), rect);
    response
}

/// Draw a slider track and handle, updating `value` on click or drag.
pub fn slider(ui: &mut Ui, rect: Rect, value: &mut f32, visuals: &Visuals) -> Response {
    let response = ui.interact(
        rect,
        ui.id().with(("mc-slider", rect.min.x as i32)),
        Sense::click_and_drag(),
    );
    let hovered = response.hovered();
    let track = if hovered {
        &visuals.textures.slider_hover
    } else {
        &visuals.textures.slider
    };
    nine_slice(
        ui.painter(),
        track,
        rect,
        1.0,
        visuals.scale as f32,
        Color32::WHITE,
    );
    let border = visuals.px(1.0);
    let handle_w = visuals.px(8.0);
    let handle_h = visuals.px(20.0);
    let inner = (rect.width() - border * 2.0 - handle_w).max(1.0);
    let x = rect.min.x + border + inner * value.clamp(0.0, 1.0);
    let handle_rect = Rect::from_min_size(
        pos2(x, rect.center().y - handle_h / 2.0),
        vec2(handle_w, handle_h),
    );
    let handle = if hovered {
        &visuals.textures.slider_handle_hover
    } else {
        &visuals.textures.slider_handle
    };
    painter_image_scaled(ui.painter(), handle.id(), handle_rect);
    if response.dragged()
        && let Some(pos) = response.interact_pointer_pos()
    {
        *value = ((pos.x - rect.min.x - border) / inner).clamp(0.0, 1.0);
    }
    response
}

/// Draw a static text field with a focus-aware border.
pub fn text_field(ui: &mut Ui, rect: Rect, text: &str, visuals: &Visuals) -> Response {
    let response = ui.interact(
        rect,
        ui.id().with(("mc-field", rect.min.x as i32)),
        Sense::click(),
    );
    let focused = response.has_focus() || response.hovered();
    let texture = if focused {
        &visuals.textures.text_field_focused
    } else {
        &visuals.textures.text_field
    };
    nine_slice(
        ui.painter(),
        texture,
        rect,
        1.0,
        visuals.scale as f32,
        Color32::WHITE,
    );
    visuals.font.draw(
        ui.painter(),
        pos2(
            rect.min.x + visuals.px(4.0),
            rect.center().y - visuals.font.line_height(visuals.scale) / 2.0,
        ),
        text,
        visuals.scale,
        Color32::from_rgb(234, 234, 234),
        true,
    );
    response
}

/// Draw a scrollbar with the supplied nine-slice sprites.
pub fn scrollbar(painter: &Painter, rect: Rect, fraction: f32, visuals: &Visuals) {
    fill(painter, &visuals.textures.scroller_bg, rect, Color32::WHITE);
    let thumb_height = (rect.height() * 0.35).max(visuals.px(12.0));
    let y = rect.min.y + (rect.height() - thumb_height) * fraction.clamp(0.0, 1.0);
    let thumb = Rect::from_min_size(pos2(rect.min.x, y), vec2(rect.width(), thumb_height));
    nine_slice(
        painter,
        &visuals.textures.scroller,
        thumb,
        1.0,
        visuals.scale as f32,
        Color32::WHITE,
    );
}

fn painter_image_scaled(painter: &Painter, id: TextureId, rect: Rect) {
    painter.image(
        id,
        rect,
        Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
        Color32::WHITE,
    );
}

/// Pick an integer GUI scale that keeps widgets readable at the current window size.
pub fn gui_scale(size: Vec2) -> u32 {
    let by_height = (size.y / 360.0).floor();
    let by_width = (size.x / 640.0).floor();
    (by_height.min(by_width) as u32).clamp(1, 3)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gui_scale_is_integer_and_bounded() {
        assert_eq!(gui_scale(vec2(800.0, 500.0)), 1);
        assert_eq!(gui_scale(vec2(1280.0, 720.0)), 2);
        assert_eq!(gui_scale(vec2(1920.0, 1080.0)), 3);
    }
}
