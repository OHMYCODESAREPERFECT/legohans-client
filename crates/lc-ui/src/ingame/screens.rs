//! Visual-only Minecraft Java Edition menu screens.

use egui::{Color32, Image, Painter, Pos2, Rect, Sense, Ui, pos2, vec2};

use super::{
    panorama::Panorama,
    state::{
        LANGUAGES, LOADING_DURATION, LanguageEntry, MenuState, SERVERS, Screen, ServerEntry,
        WORLDS, WorldEntry,
    },
    widgets::{self, Visuals},
};

const TEXT: Color32 = Color32::from_rgb(255, 255, 255);
const DIM: Color32 = Color32::from_rgb(170, 170, 170);
const GOLD: Color32 = Color32::from_rgb(255, 220, 90);

/// Dispatch one frame to the current screen.
pub fn show(
    ui: &mut Ui,
    rect: Rect,
    state: &mut MenuState,
    visuals: &Visuals,
    panorama: &mut Panorama,
) {
    match state.screen {
        Screen::Loading => loading(ui.painter(), rect, state, visuals),
        Screen::MainMenu => main_menu(ui, rect, state, visuals, panorama),
        Screen::Singleplayer => singleplayer(ui, rect, state, visuals, panorama),
        Screen::Multiplayer => multiplayer(ui, rect, state, visuals, panorama),
        Screen::CreateWorld => create_world(ui, rect, state, visuals, panorama),
        Screen::WorldSettings => world_settings(ui, rect, state, visuals, panorama),
        Screen::Options => options(ui, rect, state, visuals, panorama),
        Screen::Video => video(ui, rect, state, visuals, panorama),
        Screen::Controls => controls(ui, rect, state, visuals, panorama),
        Screen::Language => language(ui, rect, state, visuals, panorama),
        Screen::Accessibility => accessibility(ui, rect, state, visuals, panorama),
        Screen::Pause => pause(ui, rect, state, visuals),
    }
    transition(ui.painter(), rect, state);
}

fn backdrop(
    painter: &Painter,
    rect: Rect,
    visuals: &Visuals,
    panorama: &mut Panorama,
    yaw: f32,
    pitch: f32,
) {
    panorama.render(yaw, pitch, rect.size());
    panorama.paint(painter, rect, &visuals.textures.overlay);
    painter.rect_filled(
        rect,
        egui::CornerRadius::ZERO,
        Color32::from_rgba_premultiplied(0, 0, 0, 24),
    );
}

fn transition(painter: &Painter, rect: Rect, state: &MenuState) {
    if state.transition >= 1.0 {
        return;
    }
    let alpha = ((1.0 - state.transition) * 255.0).clamp(0.0, 255.0) as u8;
    painter.rect_filled(
        rect,
        egui::CornerRadius::ZERO,
        Color32::from_black_alpha(alpha),
    );
}

fn centered(rect: Rect, width: f32, height: f32, y_offset: f32) -> Rect {
    Rect::from_center_size(
        pos2(rect.center().x, rect.min.y + y_offset),
        vec2(width, height),
    )
}

fn column(rect: Rect, width: f32, height: f32, gap: f32, top: f32) -> Vec<Rect> {
    let mut y = top;
    (0..5)
        .map(|_| {
            let r =
                Rect::from_min_size(pos2(rect.center().x - width / 2.0, y), vec2(width, height));
            y += height + gap;
            r
        })
        .collect()
}

fn header(painter: &Painter, rect: Rect, state: &MenuState, visuals: &Visuals, inworld: bool) {
    let title = state.screen.title();
    if !title.is_empty() {
        let h = visuals.px(28.0);
        let title_rect = Rect::from_min_size(
            pos2(rect.min.x, rect.min.y + visuals.px(12.0)),
            vec2(rect.width(), h),
        );
        visuals
            .font
            .draw_centered(painter, title_rect, title, visuals.scale + 1, TEXT, true);
    }
    let sep_y = rect.min.y + visuals.px(44.0);
    widgets::separator(
        painter,
        Rect::from_min_size(pos2(rect.min.x, sep_y), vec2(rect.width(), visuals.px(2.0))),
        visuals,
        inworld,
        true,
    );
}

fn footer(painter: &Painter, rect: Rect, state: &MenuState, visuals: &Visuals, inworld: bool) {
    let sep_y = rect.max.y - visuals.px(18.0);
    widgets::separator(
        painter,
        Rect::from_min_size(pos2(rect.min.x, sep_y), vec2(rect.width(), visuals.px(2.0))),
        visuals,
        inworld,
        false,
    );
    visuals.font.draw(
        painter,
        pos2(rect.min.x + visuals.px(8.0), rect.max.y - visuals.px(14.0)),
        "Minecraft Java Edition 1.21.11 - visual preview",
        visuals.scale,
        DIM,
        true,
    );
    if state.can_go_back() {
        visuals.font.draw(
            painter,
            pos2(
                rect.max.x - visuals.px(120.0),
                rect.max.y - visuals.px(14.0),
            ),
            "ESC: Back",
            visuals.scale,
            DIM,
            true,
        );
    }
}

fn draw_image(painter: &Painter, texture: &egui::TextureHandle, rect: Rect) {
    widgets::fill(painter, texture, rect, Color32::WHITE);
}

fn main_menu(
    ui: &mut Ui,
    rect: Rect,
    state: &mut MenuState,
    visuals: &Visuals,
    panorama: &mut Panorama,
) {
    backdrop(
        ui.painter(),
        rect,
        visuals,
        panorama,
        state.yaw(),
        state.pitch(),
    );

    // Wordmark and the custom edition label.
    let logo_w = rect.width() * 0.615;
    let logo_h = logo_w * 176.0 / 1024.0;
    let logo_center_y = rect.min.y + rect.height() * 0.19;
    let logo_rect = centered(rect, logo_w, logo_h, logo_center_y - rect.min.y);
    draw_image(ui.painter(), &visuals.textures.logo, logo_rect);

    let edition_w = rect.width() * 0.29;
    let edition_h = edition_w * 64.0 / 512.0;
    let edition_rect = Rect::from_center_size(
        pos2(rect.center().x, logo_rect.max.y + visuals.px(7.0)),
        vec2(edition_w, edition_h),
    );
    draw_image(ui.painter(), &visuals.textures.edition, edition_rect);

    // Yellow bouncing phrase, rotated over the right side of the logo.
    let splash_center = pos2(
        rect.center().x + logo_w * 0.45,
        logo_rect.center().y + visuals.px(18.0) + super::splash::SplashText::bounce(state.time),
    );
    let splash_scale = (rect.width() / 1280.0).clamp(0.45, 1.0);
    let splash_rect = Rect::from_center_size(
        splash_center,
        vec2(
            visuals.splash.width * splash_scale,
            visuals.splash.height * splash_scale,
        ),
    );
    let splash_image = Image::new(&visuals.splash.texture)
        .fit_to_exact_size(splash_rect.size())
        .rotate(super::splash::SplashText::angle(), vec2(0.5, 0.5));
    ui.put(splash_rect, splash_image);

    // Three large vanilla-style menu buttons.
    let button_w = visuals.px(297.0);
    let button_h = visuals.px(29.0);
    let gap = visuals.px(7.0);
    let start_y = rect.min.y + rect.height() * 0.443;
    let main_rects = [
        Rect::from_min_size(
            pos2(rect.center().x - button_w / 2.0, start_y),
            vec2(button_w, button_h),
        ),
        Rect::from_min_size(
            pos2(rect.center().x - button_w / 2.0, start_y + button_h + gap),
            vec2(button_w, button_h),
        ),
        Rect::from_min_size(
            pos2(
                rect.center().x - button_w / 2.0,
                start_y + (button_h + gap) * 2.0,
            ),
            vec2(button_w, button_h),
        ),
    ];
    let labels = ["Singleplayer", "Multiplayer", "Minecraft Realms"];
    let targets = [
        Screen::Singleplayer,
        Screen::Multiplayer,
        Screen::Multiplayer,
    ];
    for (button_rect, (label, target)) in
        main_rects.into_iter().zip(labels.into_iter().zip(targets))
    {
        if widgets::button(ui, button_rect, label, visuals, true).clicked() {
            state.go(target);
        }
    }
    draw_realms_icons(ui.painter(), main_rects[2], visuals);

    // Bottom row: icon / Options / Quit Game / icon.
    let small_w = visuals.px(31.0);
    let wide_w = visuals.px(146.0);
    let row_gap = visuals.px(3.0);
    let mid_gap = visuals.px(10.0);
    let bottom_h = visuals.px(32.0);
    let group_w = small_w * 2.0 + wide_w * 2.0 + row_gap * 2.0 + mid_gap;
    let row_y = rect.min.y + rect.height() * 0.808;
    let row_x = rect.center().x - group_w / 2.0;
    let small_left = Rect::from_min_size(pos2(row_x, row_y), vec2(small_w, bottom_h));
    let options = Rect::from_min_size(
        pos2(row_x + small_w + row_gap, row_y),
        vec2(wide_w, bottom_h),
    );
    let quit = Rect::from_min_size(
        pos2(row_x + small_w + row_gap + wide_w + mid_gap, row_y),
        vec2(wide_w, bottom_h),
    );
    let small_right = Rect::from_min_size(
        pos2(
            row_x + small_w + row_gap + wide_w + mid_gap + wide_w + row_gap,
            row_y,
        ),
        vec2(small_w, bottom_h),
    );
    widgets::button(ui, small_left, "", visuals, true);
    if widgets::button(ui, options, "Options...", visuals, true).clicked() {
        state.go(Screen::Options);
    }
    if widgets::button(ui, quit, "Quit Game", visuals, true).clicked() {
        state.go(Screen::MainMenu);
    }
    widgets::button(ui, small_right, "", visuals, true);
    draw_bottom_icons(ui.painter(), small_left, small_right, visuals);

    // Vanilla-style footer text.
    let font_scale = visuals.scale + 2;
    let font_y = rect.max.y - visuals.px(15.0);
    visuals.font.draw(
        ui.painter(),
        pos2(rect.min.x + visuals.px(3.0), font_y),
        "Minecraft 1.21.11",
        font_scale,
        DIM,
        true,
    );
    let copyright = "Copyright Mojang AB. Do not distribute!";
    let copyright_w = visuals.font.width(copyright, font_scale);
    visuals.font.draw(
        ui.painter(),
        pos2(rect.max.x - copyright_w - visuals.px(5.0), font_y),
        copyright,
        font_scale,
        DIM,
        true,
    );
}

fn draw_realms_icons(painter: &Painter, button: Rect, visuals: &Visuals) {
    let unit = visuals.px(2.0);
    let y = button.center().y;
    let gem_x = button.max.x - visuals.px(34.0);
    let gem = [
        (0, -3, Color32::from_rgb(150, 255, 225)),
        (-1, -2, Color32::from_rgb(80, 225, 195)),
        (0, -2, Color32::from_rgb(180, 255, 235)),
        (1, -2, Color32::from_rgb(45, 170, 155)),
        (-2, -1, Color32::from_rgb(45, 150, 140)),
        (-1, -1, Color32::from_rgb(100, 240, 210)),
        (0, -1, Color32::from_rgb(200, 255, 240)),
        (1, -1, Color32::from_rgb(70, 210, 185)),
        (2, -1, Color32::from_rgb(25, 120, 115)),
        (-2, 0, Color32::from_rgb(30, 130, 125)),
        (-1, 0, Color32::from_rgb(90, 225, 195)),
        (0, 0, Color32::from_rgb(170, 255, 230)),
        (1, 0, Color32::from_rgb(60, 195, 170)),
        (2, 0, Color32::from_rgb(20, 110, 105)),
        (-1, 1, Color32::from_rgb(35, 140, 130)),
        (0, 1, Color32::from_rgb(110, 235, 205)),
        (1, 1, Color32::from_rgb(30, 125, 120)),
        (0, 2, Color32::from_rgb(25, 115, 110)),
    ];
    for (dx, dy, color) in gem {
        painter.rect_filled(
            Rect::from_min_size(
                pos2(gem_x + dx as f32 * unit, y + dy as f32 * unit),
                vec2(unit, unit),
            ),
            egui::CornerRadius::ZERO,
            color,
        );
    }

    let book_x = button.max.x - visuals.px(18.0);
    let book_y = y - visuals.px(8.0);
    let book_w = visuals.px(15.0);
    let book_h = visuals.px(16.0);
    painter.rect_filled(
        Rect::from_min_size(
            pos2(book_x - visuals.px(1.0), book_y - visuals.px(1.0)),
            vec2(book_w + visuals.px(2.0), book_h + visuals.px(2.0)),
        ),
        egui::CornerRadius::ZERO,
        Color32::from_rgb(25, 25, 25),
    );
    painter.rect_filled(
        Rect::from_min_size(pos2(book_x, book_y), vec2(book_w, book_h)),
        egui::CornerRadius::ZERO,
        Color32::from_rgb(235, 232, 220),
    );
    painter.rect_filled(
        Rect::from_min_size(pos2(book_x, book_y), vec2(visuals.px(4.0), book_h)),
        egui::CornerRadius::ZERO,
        Color32::from_rgb(150, 55, 45),
    );
    for i in 1..4 {
        painter.rect_filled(
            Rect::from_min_size(
                pos2(
                    book_x + visuals.px(5.0),
                    book_y + visuals.px(2.0 + i as f32 * 3.0),
                ),
                vec2(book_w - visuals.px(6.0), visuals.px(1.0)),
            ),
            egui::CornerRadius::ZERO,
            Color32::from_rgb(150, 150, 145),
        );
    }
}

fn draw_bottom_icons(painter: &Painter, left: Rect, right: Rect, visuals: &Visuals) {
    let unit = visuals.px(2.0);
    let lc = left.center();
    let ring = [
        (-2, -1),
        (-1, -2),
        (0, -2),
        (1, -2),
        (2, -1),
        (2, 0),
        (2, 1),
        (1, 2),
        (0, 2),
        (-1, 2),
        (-2, 1),
        (-2, 0),
    ];
    for (dx, dy) in ring {
        painter.rect_filled(
            Rect::from_min_size(
                pos2(lc.x + dx as f32 * unit, lc.y + dy as f32 * unit),
                vec2(unit, unit),
            ),
            egui::CornerRadius::ZERO,
            Color32::from_rgb(60, 120, 180),
        );
    }
    painter.rect_filled(
        Rect::from_min_size(pos2(lc.x - unit, lc.y - unit), vec2(unit * 2.0, unit * 2.0)),
        egui::CornerRadius::ZERO,
        Color32::from_rgb(230, 230, 220),
    );

    let rc = right.center();
    for (dx, dy) in [
        (-1, -2),
        (0, -2),
        (1, -2),
        (-2, -1),
        (2, -1),
        (-1, 0),
        (0, 0),
        (1, 0),
        (-1, 1),
        (1, 1),
        (-1, 2),
        (1, 2),
    ] {
        painter.rect_filled(
            Rect::from_min_size(
                pos2(rc.x + dx as f32 * unit, rc.y + dy as f32 * unit),
                vec2(unit, unit),
            ),
            egui::CornerRadius::ZERO,
            Color32::from_rgb(225, 225, 220),
        );
    }
}

fn loading(painter: &Painter, rect: Rect, state: &MenuState, visuals: &Visuals) {
    painter.rect_filled(
        rect,
        egui::CornerRadius::ZERO,
        Color32::from_rgb(16, 16, 18),
    );
    let t = state.time;
    let center = rect.center();
    draw_pixel_loading_art(painter, center, t, visuals);

    let logo_size = visuals.px(160.0);
    let logo_rect = Rect::from_center_size(
        pos2(center.x, center.y - visuals.px(48.0)),
        vec2(logo_size, logo_size),
    );
    draw_image(painter, &visuals.textures.mojang, logo_rect);

    let bar_w = visuals.px(220.0);
    let bar_h = visuals.px(14.0);
    let bar = Rect::from_center_size(
        pos2(center.x, center.y + visuals.px(78.0)),
        vec2(bar_w, bar_h),
    );
    painter.rect_filled(bar, egui::CornerRadius::ZERO, Color32::from_rgb(32, 32, 36));
    let progress = (t / LOADING_DURATION).clamp(0.0, 1.0).max(0.04);
    let fill_w = bar.width() * progress;
    let gradient_steps = 10;
    for i in 0..gradient_steps {
        let x0 = bar.min.x + fill_w * i as f32 / gradient_steps as f32;
        let x1 = bar.min.x + fill_w * (i + 1) as f32 / gradient_steps as f32;
        let shade = 150 + (i * 8).min(90);
        painter.rect_filled(
            Rect::from_min_max(pos2(x0, bar.min.y), pos2(x1, bar.max.y)),
            egui::CornerRadius::ZERO,
            Color32::from_rgb(shade, shade, shade),
        );
    }
    painter.rect_stroke(
        bar,
        egui::CornerRadius::ZERO,
        (1.0, Color32::from_gray(90)),
        egui::StrokeKind::Outside,
    );

    let splash = [
        "Preparing the world...",
        "Loading visual menus...",
        "Placing blocks...",
    ];
    let index = (t as usize / 2) % splash.len();
    visuals.font.draw_centered(
        painter,
        Rect::from_center_size(
            pos2(center.x, bar.max.y + visuals.px(22.0)),
            vec2(bar_w, visuals.px(12.0)),
        ),
        splash[index],
        visuals.scale,
        DIM,
        true,
    );
    visuals.font.draw_centered(
        painter,
        Rect::from_center_size(
            pos2(center.x, center.y + visuals.px(118.0)),
            vec2(bar_w, visuals.px(12.0)),
        ),
        "MOJANG STUDIOS",
        visuals.scale,
        GOLD,
        true,
    );
}

fn draw_pixel_loading_art(painter: &Painter, center: Pos2, time: f32, visuals: &Visuals) {
    let unit = visuals.px(16.0);
    for y in -5..5 {
        for x in -9..9 {
            let phase = (x * 7 + y * 11) as f32 * 0.37;
            let alpha = (((time * 0.8 + phase).sin() + 1.0) * 22.0) as u8;
            painter.rect_filled(
                Rect::from_min_size(
                    pos2(center.x + x as f32 * unit, center.y + (y + 8) as f32 * unit),
                    vec2(unit - 1.0, unit - 1.0),
                ),
                egui::CornerRadius::ZERO,
                Color32::from_rgba_premultiplied(60, 70, 90, alpha),
            );
        }
    }
}

fn list_screen_frame(
    ui: &mut Ui,
    rect: Rect,
    state: &mut MenuState,
    visuals: &Visuals,
    panorama: &mut Panorama,
    inworld: bool,
) -> (Rect, Rect, Rect) {
    backdrop(
        ui.painter(),
        rect,
        visuals,
        panorama,
        state.yaw(),
        state.pitch(),
    );
    header(ui.painter(), rect, state, visuals, inworld);
    footer(ui.painter(), rect, state, visuals, inworld);

    let top = rect.min.y + visuals.px(52.0);
    let bottom = rect.max.y - visuals.px(26.0);
    let side = visuals.px(10.0);
    let gap = visuals.px(8.0);
    let list_w = ((rect.width() - side * 2.0 - gap) * 0.62).floor();
    let list_rect = Rect::from_min_max(
        pos2(rect.min.x + side, top),
        pos2(rect.min.x + side + list_w, bottom),
    );
    let detail_rect = Rect::from_min_max(
        pos2(list_rect.max.x + gap, top),
        pos2(rect.max.x - side, bottom),
    );
    let buttons_rect = Rect::from_min_max(
        pos2(rect.min.x + side, top),
        pos2(rect.max.x - side, bottom),
    );
    (list_rect, detail_rect, buttons_rect)
}

fn singleplayer(
    ui: &mut Ui,
    rect: Rect,
    state: &mut MenuState,
    visuals: &Visuals,
    panorama: &mut Panorama,
) {
    let (list_rect, detail_rect, buttons_rect) =
        list_screen_frame(ui, rect, state, visuals, panorama, false);
    widgets::list_background(ui.painter(), list_rect, visuals, false);
    let row_h = visuals.px(36.0);
    let scroll_w = visuals.px(6.0);
    for (i, world) in WORLDS.iter().enumerate() {
        let row = Rect::from_min_size(
            pos2(
                list_rect.min.x + visuals.px(2.0),
                list_rect.min.y + visuals.px(2.0) + i as f32 * (row_h + visuals.px(2.0)),
            ),
            vec2(list_rect.width() - scroll_w - visuals.px(6.0), row_h),
        );
        if row.max.y > list_rect.max.y {
            break;
        }
        let selected = i == state.selected_world;
        let response = ui.interact(row, ui.id().with(("world-row", i)), Sense::click());
        if selected {
            ui.painter().rect_filled(
                row,
                egui::CornerRadius::ZERO,
                Color32::from_rgba_premultiplied(90, 110, 150, 110),
            );
        } else if response.hovered() {
            ui.painter().rect_filled(
                row,
                egui::CornerRadius::ZERO,
                Color32::from_rgba_premultiplied(80, 80, 90, 80),
            );
        }
        if response.clicked() {
            state.selected_world = i;
        }
        draw_world_icon(
            ui.painter(),
            world,
            visuals,
            row.min + vec2(visuals.px(2.0), visuals.px(2.0)),
        );
        visuals.font.draw(
            ui.painter(),
            row.min + vec2(visuals.px(22.0), visuals.px(4.0)),
            world.name,
            visuals.scale,
            TEXT,
            true,
        );
        visuals.font.draw(
            ui.painter(),
            row.min + vec2(visuals.px(22.0), visuals.px(20.0)),
            world.detail,
            visuals.scale,
            DIM,
            true,
        );
    }
    let bar = Rect::from_min_size(
        pos2(
            list_rect.max.x - scroll_w - visuals.px(2.0),
            list_rect.min.y + visuals.px(2.0),
        ),
        vec2(scroll_w, list_rect.height() - visuals.px(4.0)),
    );
    widgets::scrollbar(ui.painter(), bar, 0.0, visuals);

    widgets::panel(ui.painter(), detail_rect, visuals, false);
    let selected = &WORLDS[state.selected_world];
    visuals.font.draw(
        ui.painter(),
        detail_rect.min + vec2(visuals.px(10.0), visuals.px(12.0)),
        selected.name,
        visuals.scale + 1,
        TEXT,
        true,
    );
    let info = [
        "Visual placeholder world".to_string(),
        selected.detail.to_string(),
        format!("Last played: {} days ago", selected.icon * 3 + 1),
        "No world data is loaded.".to_string(),
    ];
    for (i, line) in info.iter().enumerate() {
        visuals.font.draw(
            ui.painter(),
            detail_rect.min + vec2(visuals.px(10.0), visuals.px(42.0 + i as f32 * 16.0)),
            line,
            visuals.scale,
            DIM,
            true,
        );
    }

    let bw = visuals.px(200.0);
    let bh = visuals.px(20.0);
    let gap = visuals.px(4.0);
    let by = buttons_rect.max.y - (bh * 3.0 + gap * 2.0);
    let left = buttons_rect.min.x + visuals.px(2.0);
    let play = Rect::from_min_size(pos2(left, by), vec2(bw, bh));
    let create = Rect::from_min_size(pos2(left, by + bh + gap), vec2(bw, bh));
    let back = Rect::from_min_size(pos2(left, by + (bh + gap) * 2.0), vec2(bw, bh));
    // Visual-only: world loading is intentionally not implemented.
    widgets::button(ui, play, "Play Selected World", visuals, true);
    if widgets::button(ui, create, "Create New World", visuals, true).clicked() {
        state.go(Screen::CreateWorld);
    }
    if widgets::button(ui, back, "Back", visuals, true).clicked() {
        state.back();
    }
    let delete = Rect::from_min_size(
        pos2(buttons_rect.max.x - bw - visuals.px(2.0), by),
        vec2(bw, bh),
    );
    widgets::button(ui, delete, "Delete", visuals, false);
}

fn draw_world_icon(painter: &Painter, world: &WorldEntry, visuals: &Visuals, at: Pos2) {
    let texture = if world.icon == 4 {
        &visuals.textures.world_warning
    } else {
        &visuals.textures.world_join
    };
    let size = visuals.px(18.0);
    widgets::fill(
        painter,
        texture,
        Rect::from_min_size(at, vec2(size, size)),
        Color32::WHITE,
    );
    if world.name.contains("Hardcore") {
        let selected = visuals.px(18.0);
        widgets::fill(
            painter,
            &visuals.textures.world_selected,
            Rect::from_min_size(at + vec2(size, 0.0), vec2(selected, selected)),
            Color32::WHITE,
        );
    }
}

fn multiplayer(
    ui: &mut Ui,
    rect: Rect,
    state: &mut MenuState,
    visuals: &Visuals,
    panorama: &mut Panorama,
) {
    let (list_rect, detail_rect, buttons_rect) =
        list_screen_frame(ui, rect, state, visuals, panorama, false);
    widgets::list_background(ui.painter(), list_rect, visuals, false);
    let row_h = visuals.px(38.0);
    for (i, server) in SERVERS.iter().enumerate() {
        let row = Rect::from_min_size(
            pos2(
                list_rect.min.x + visuals.px(2.0),
                list_rect.min.y + visuals.px(2.0) + i as f32 * (row_h + visuals.px(2.0)),
            ),
            vec2(list_rect.width() - visuals.px(8.0), row_h),
        );
        if row.max.y > list_rect.max.y {
            break;
        }
        let selected = i == state.selected_server;
        let response = ui.interact(row, ui.id().with(("server-row", i)), Sense::click());
        if selected {
            ui.painter().rect_filled(
                row,
                egui::CornerRadius::ZERO,
                Color32::from_rgba_premultiplied(90, 110, 150, 110),
            );
        } else if response.hovered() {
            ui.painter().rect_filled(
                row,
                egui::CornerRadius::ZERO,
                Color32::from_rgba_premultiplied(80, 80, 90, 80),
            );
        }
        if response.clicked() {
            state.selected_server = i;
        }
        draw_ping(
            ui.painter(),
            server,
            visuals,
            row.min + vec2(visuals.px(3.0), visuals.px(4.0)),
        );
        visuals.font.draw(
            ui.painter(),
            row.min + vec2(visuals.px(18.0), visuals.px(3.0)),
            server.name,
            visuals.scale,
            TEXT,
            true,
        );
        visuals.font.draw(
            ui.painter(),
            row.min + vec2(visuals.px(18.0), visuals.px(18.0)),
            server.address,
            visuals.scale,
            DIM,
            true,
        );
    }
    let bar = Rect::from_min_size(
        pos2(
            list_rect.max.x - visuals.px(8.0),
            list_rect.min.y + visuals.px(2.0),
        ),
        vec2(visuals.px(6.0), list_rect.height() - visuals.px(4.0)),
    );
    widgets::scrollbar(ui.painter(), bar, 0.15, visuals);

    widgets::panel(ui.painter(), detail_rect, visuals, false);
    let selected = &SERVERS[state.selected_server];
    visuals.font.draw(
        ui.painter(),
        detail_rect.min + vec2(visuals.px(10.0), visuals.px(12.0)),
        selected.name,
        visuals.scale + 1,
        TEXT,
        true,
    );
    for (i, line) in [
        selected.motd,
        selected.address,
        selected.players,
        "No connection is made in this preview.",
    ]
    .iter()
    .enumerate()
    {
        visuals.font.draw(
            ui.painter(),
            detail_rect.min + vec2(visuals.px(10.0), visuals.px(42.0 + i as f32 * 16.0)),
            line,
            visuals.scale,
            DIM,
            true,
        );
    }

    let bw = visuals.px(200.0);
    let bh = visuals.px(20.0);
    let gap = visuals.px(4.0);
    let by = buttons_rect.max.y - (bh * 3.0 + gap * 2.0);
    let left = buttons_rect.min.x + visuals.px(2.0);
    // Visual-only: server connections are intentionally not implemented.
    widgets::button(
        ui,
        Rect::from_min_size(pos2(left, by), vec2(bw, bh)),
        "Play Selected Server",
        visuals,
        true,
    );
    if widgets::button(
        ui,
        Rect::from_min_size(pos2(left, by + bh + gap), vec2(bw, bh)),
        "Add Server",
        visuals,
        true,
    )
    .clicked()
    {
        state.go(Screen::Multiplayer);
    }
    if widgets::button(
        ui,
        Rect::from_min_size(pos2(left, by + (bh + gap) * 2.0), vec2(bw, bh)),
        "Back",
        visuals,
        true,
    )
    .clicked()
    {
        state.back();
    }
    widgets::button(
        ui,
        Rect::from_min_size(
            pos2(
                buttons_rect.max.x - bw - visuals.px(2.0),
                by + (bh + gap) * 2.0,
            ),
            vec2(bw, bh),
        ),
        "Refresh",
        visuals,
        true,
    );
}

fn draw_ping(painter: &Painter, server: &ServerEntry, visuals: &Visuals, at: Pos2) {
    let size = visuals.px(12.0);
    let texture = if server.ping == 0 {
        &visuals.textures.server_unreachable
    } else {
        &visuals.textures.ping[server.ping - 1]
    };
    widgets::fill(
        painter,
        texture,
        Rect::from_min_size(at, vec2(size, size)),
        Color32::WHITE,
    );
}

fn world_editor_scaffold(
    ui: &mut Ui,
    rect: Rect,
    state: &mut MenuState,
    visuals: &Visuals,
    panorama: &mut Panorama,
    settings_mode: bool,
) {
    backdrop(
        ui.painter(),
        rect,
        visuals,
        panorama,
        state.yaw(),
        state.pitch(),
    );
    header(ui.painter(), rect, state, visuals, false);
    footer(ui.painter(), rect, state, visuals, false);

    let top = rect.min.y + visuals.px(54.0);
    let bottom = rect.max.y - visuals.px(26.0);
    let side = visuals.px(12.0);
    let gap = visuals.px(8.0);
    let left_w = (rect.width() - side * 2.0 - gap) * 0.42;
    let left = Rect::from_min_max(
        pos2(rect.min.x + side, top),
        pos2(rect.min.x + side + left_w, bottom),
    );
    let right = Rect::from_min_max(pos2(left.max.x + gap, top), pos2(rect.max.x - side, bottom));
    widgets::panel(ui.painter(), left, visuals, false);
    widgets::panel(ui.painter(), right, visuals, false);

    let field_w = left.width() - visuals.px(20.0);
    let field_h = visuals.px(20.0);
    let mut y = left.min.y + visuals.px(24.0);
    widgets::label(
        ui.painter(),
        "World Name",
        left.min + vec2(visuals.px(10.0), y),
        visuals,
        TEXT,
    );
    y += visuals.px(12.0);
    let name_rect = Rect::from_min_size(
        pos2(left.min.x + visuals.px(10.0), y),
        vec2(field_w, field_h),
    );
    widgets::text_field(ui, name_rect, &state.world_name, visuals);
    y += field_h + visuals.px(12.0);
    widgets::label(
        ui.painter(),
        "Seed",
        left.min + vec2(visuals.px(10.0), y),
        visuals,
        TEXT,
    );
    y += visuals.px(12.0);
    let seed_text = if state.seed.is_empty() {
        "[leave blank for random]"
    } else {
        &state.seed
    };
    let seed_rect = Rect::from_min_size(
        pos2(left.min.x + visuals.px(10.0), y),
        vec2(field_w, field_h),
    );
    widgets::text_field(ui, seed_rect, seed_text, visuals);
    y += field_h + visuals.px(18.0);
    let bw = field_w.min(visuals.px(200.0));
    let world_type = format!(
        "World Type: {}",
        ["Default", "Flat", "Amplified", "Customized"][state.settings.world_type as usize]
    );
    if widgets::button(
        ui,
        Rect::from_min_size(
            pos2(left.min.x + visuals.px(10.0), y),
            vec2(bw, visuals.px(20.0)),
        ),
        &world_type,
        visuals,
        true,
    )
    .clicked()
    {
        state.settings.world_type = (state.settings.world_type + 1) % 4;
    }
    y += visuals.px(26.0);
    let structures = if state.settings.allow_cheats {
        "Structures: On"
    } else {
        "Structures: Off"
    };
    widgets::label(
        ui.painter(),
        structures,
        left.min + vec2(visuals.px(10.0), y),
        visuals,
        DIM,
    );

    let tab_y = right.min.y + visuals.px(8.0);
    widgets::label(
        ui.painter(),
        if settings_mode {
            "World Settings"
        } else {
            "Create World Settings"
        },
        right.min + vec2(visuals.px(12.0), tab_y),
        visuals,
        GOLD,
    );
    widgets::separator(
        ui.painter(),
        Rect::from_min_size(
            pos2(right.min.x, tab_y + visuals.px(14.0)),
            vec2(right.width(), visuals.px(2.0)),
        ),
        visuals,
        false,
        true,
    );

    let row_h = visuals.px(30.0);
    let mut ry = tab_y + visuals.px(24.0);
    let difficulty = ["Peaceful", "Easy", "Normal", "Hard"][state.settings.difficulty as usize];
    draw_setting_button(
        ui,
        right,
        visuals,
        CycleSetting {
            y: &mut ry,
            row_h,
            text: &format!("Difficulty: {difficulty}"),
            value: &mut state.settings.difficulty,
            variants: 4,
        },
    );
    let gamemode =
        ["Survival", "Creative", "Adventure", "Spectator"][state.settings.world_type as usize % 4];
    draw_setting_button(
        ui,
        right,
        visuals,
        CycleSetting {
            y: &mut ry,
            row_h,
            text: &format!("Game Mode: {gamemode}"),
            value: &mut state.settings.world_type,
            variants: 4,
        },
    );
    let cheat_rect = Rect::from_min_size(
        pos2(right.min.x + visuals.px(12.0), ry),
        vec2(visuals.px(20.0), visuals.px(20.0)),
    );
    if widgets::checkbox(ui, cheat_rect, state.settings.allow_cheats, visuals).clicked() {
        state.settings.allow_cheats = !state.settings.allow_cheats;
    }
    widgets::label(
        ui.painter(),
        "Allow Cheats",
        cheat_rect.max + vec2(visuals.px(8.0), -visuals.px(16.0)),
        visuals,
        TEXT,
    );
    ry += row_h;
    widgets::label(
        ui.painter(),
        "World generation is not performed in this visual preview.",
        right.min + vec2(visuals.px(12.0), ry),
        visuals,
        DIM,
    );

    let bh = visuals.px(20.0);
    let bw = visuals.px(200.0).min(right.width() - visuals.px(24.0));
    let by = bottom - bh - visuals.px(6.0);
    if widgets::button(
        ui,
        Rect::from_min_size(
            pos2(right.min.x + visuals.px(12.0), by - bh - visuals.px(4.0)),
            vec2(bw, bh),
        ),
        if settings_mode {
            "Done"
        } else {
            "Create New World"
        },
        visuals,
        true,
    )
    .clicked()
    {
        state.go(Screen::Pause);
    }
    if widgets::button(
        ui,
        Rect::from_min_size(pos2(right.min.x + visuals.px(12.0), by), vec2(bw, bh)),
        "Cancel",
        visuals,
        true,
    )
    .clicked()
    {
        state.back();
    }
    if !settings_mode
        && widgets::button(
            ui,
            Rect::from_min_size(
                pos2(left.min.x + visuals.px(10.0), bottom - bh - visuals.px(6.0)),
                vec2(bw, bh),
            ),
            "More Options",
            visuals,
            true,
        )
        .clicked()
    {
        state.go(Screen::WorldSettings);
    }
}

/// Parameters for one cycling world-setting row.
struct CycleSetting<'a> {
    /// Current vertical cursor, advanced after drawing.
    y: &'a mut f32,
    /// Row height in screen pixels.
    row_h: f32,
    /// Button label.
    text: &'a str,
    /// Value advanced by one variant on click.
    value: &'a mut u8,
    /// Number of possible values.
    variants: u8,
}

fn draw_setting_button(ui: &mut Ui, area: Rect, visuals: &Visuals, setting: CycleSetting<'_>) {
    let bw = area.width() - visuals.px(24.0);
    if widgets::button(
        ui,
        Rect::from_min_size(
            pos2(area.min.x + visuals.px(12.0), *setting.y),
            vec2(bw, visuals.px(20.0)),
        ),
        setting.text,
        visuals,
        true,
    )
    .clicked()
    {
        *setting.value = (*setting.value + 1) % setting.variants;
    }
    *setting.y += setting.row_h;
}

fn create_world(
    ui: &mut Ui,
    rect: Rect,
    state: &mut MenuState,
    visuals: &Visuals,
    panorama: &mut Panorama,
) {
    world_editor_scaffold(ui, rect, state, visuals, panorama, false);
}

fn world_settings(
    ui: &mut Ui,
    rect: Rect,
    state: &mut MenuState,
    visuals: &Visuals,
    panorama: &mut Panorama,
) {
    world_editor_scaffold(ui, rect, state, visuals, panorama, true);
}

fn options(
    ui: &mut Ui,
    rect: Rect,
    state: &mut MenuState,
    visuals: &Visuals,
    panorama: &mut Panorama,
) {
    full_menu_scaffold(ui, rect, state, visuals, panorama);
    let bw = visuals.px(200.0);
    let bh = visuals.px(20.0);
    let gap = visuals.px(4.0);
    let body_top = rect.min.y + visuals.px(64.0);
    let rects = column(rect, bw, bh, gap, body_top);
    let items = [
        ("Video Settings...", Screen::Video),
        ("Controls...", Screen::Controls),
        ("Language...", Screen::Language),
        ("Accessibility Settings...", Screen::Accessibility),
        ("Done", Screen::MainMenu),
    ];
    for (r, (text, target)) in rects.into_iter().zip(items) {
        if widgets::button(ui, r, text, visuals, true).clicked() {
            if target == Screen::MainMenu {
                state.back();
            } else {
                state.go(target);
            }
        }
    }
    let note_rect = Rect::from_center_size(
        pos2(rect.center().x, rect.max.y - visuals.px(34.0)),
        vec2(bw, visuals.px(12.0)),
    );
    visuals.font.draw_centered(
        ui.painter(),
        note_rect,
        "Visual-only settings",
        visuals.scale,
        DIM,
        true,
    );
}

fn full_menu_scaffold(
    ui: &mut Ui,
    rect: Rect,
    state: &mut MenuState,
    visuals: &Visuals,
    panorama: &mut Panorama,
) {
    backdrop(
        ui.painter(),
        rect,
        visuals,
        panorama,
        state.yaw(),
        state.pitch(),
    );
    header(ui.painter(), rect, state, visuals, false);
    footer(ui.painter(), rect, state, visuals, false);
    let panel = Rect::from_center_size(
        rect.center(),
        vec2(
            (visuals.px(320.0)).min(rect.width() - visuals.px(20.0)),
            (rect.height() - visuals.px(70.0)).max(visuals.px(120.0)),
        ),
    );
    widgets::panel(ui.painter(), panel, visuals, false);
}

fn settings_row(
    ui: &mut Ui,
    area: Rect,
    y: f32,
    label: &str,
    visuals: &Visuals,
    control: impl FnOnce(&mut Ui, Rect),
) {
    let line = Rect::from_min_size(
        pos2(area.min.x + visuals.px(12.0), y + visuals.px(4.0)),
        vec2(area.width() * 0.45, visuals.px(14.0)),
    );
    visuals
        .font
        .draw(ui.painter(), line.min, label, visuals.scale, TEXT, true);
    let control_rect = Rect::from_min_size(
        pos2(area.center().x + visuals.px(10.0), y),
        vec2(area.width() * 0.44, visuals.px(20.0)),
    );
    control(ui, control_rect);
}

fn video(
    ui: &mut Ui,
    rect: Rect,
    state: &mut MenuState,
    visuals: &Visuals,
    panorama: &mut Panorama,
) {
    full_menu_scaffold(ui, rect, state, visuals, panorama);
    let area = Rect::from_center_size(
        rect.center(),
        vec2(
            (visuals.px(320.0)).min(rect.width() - visuals.px(20.0)),
            (rect.height() - visuals.px(70.0)).max(visuals.px(120.0)),
        ),
    );
    let mut y = area.min.y + visuals.px(16.0);
    let step = visuals.px(32.0);
    let graphics = ["Fast", "Balanced", "Fancy"];
    settings_row(ui, area, y, "Graphics", visuals, |ui, r| {
        let text = format!("Graphics: {}", graphics[state.settings.graphics as usize]);
        if widgets::button(ui, r, &text, visuals, true).clicked() {
            state.settings.graphics = (state.settings.graphics + 1) % 3;
        }
    });
    y += step;
    settings_row(ui, area, y, "Render Distance", visuals, |ui, r| {
        let mut distance =
            (state.settings.render_distance.saturating_sub(2) as f32 / 30.0).min(1.0);
        widgets::slider(ui, r, &mut distance, visuals);
        state.settings.render_distance = 2 + (distance * 30.0).round().clamp(0.0, 30.0) as u8;
    });
    y += step;
    settings_row(ui, area, y, "Smooth Lighting", visuals, |ui, r| {
        widgets::slider(ui, r, &mut state.settings.smooth_lighting, visuals);
    });
    y += step;
    settings_row(ui, area, y, "GUI Scale", visuals, |ui, r| {
        let labels = ["Auto", "1", "2", "3"];
        let text = format!("GUI Scale: {}", labels[state.settings.gui_scale as usize]);
        if widgets::button(ui, r, &text, visuals, true).clicked() {
            state.settings.gui_scale = (state.settings.gui_scale + 1) % labels.len() as u8;
        }
    });
    y += step;
    settings_row(ui, area, y, "VSync", visuals, |ui, r| {
        let check = Rect::from_min_size(r.min, vec2(visuals.px(20.0), visuals.px(20.0)));
        if widgets::checkbox(ui, check, state.settings.vsync, visuals).clicked() {
            state.settings.vsync = !state.settings.vsync;
        }
    });
    y += step;
    settings_row(ui, area, y, "Fullscreen", visuals, |ui, r| {
        let check = Rect::from_min_size(r.min, vec2(visuals.px(20.0), visuals.px(20.0)));
        if widgets::checkbox(ui, check, state.settings.fullscreen, visuals).clicked() {
            state.settings.fullscreen = !state.settings.fullscreen;
        }
    });
    let bw = visuals.px(200.0);
    if widgets::button(
        ui,
        Rect::from_center_size(
            pos2(rect.center().x, area.max.y - visuals.px(14.0)),
            vec2(bw, visuals.px(20.0)),
        ),
        "Done",
        visuals,
        true,
    )
    .clicked()
    {
        state.back();
    }
}

fn controls(
    ui: &mut Ui,
    rect: Rect,
    state: &mut MenuState,
    visuals: &Visuals,
    panorama: &mut Panorama,
) {
    full_menu_scaffold(ui, rect, state, visuals, panorama);
    let area = Rect::from_center_size(
        rect.center(),
        vec2(
            (visuals.px(360.0)).min(rect.width() - visuals.px(20.0)),
            (rect.height() - visuals.px(70.0)).max(visuals.px(120.0)),
        ),
    );
    widgets::list_background(ui.painter(), area, visuals, false);
    let binds = [
        ("Forward", "W"),
        ("Left", "A"),
        ("Backward", "S"),
        ("Right", "D"),
        ("Jump", "Space"),
        ("Sneak", "Shift"),
        ("Sprint", "Ctrl"),
        ("Attack", "LMB"),
        ("Use Item", "RMB"),
        ("Inventory", "E"),
        ("Drop Item", "Q"),
        ("Pause Menu", "Esc"),
    ];
    let row_h = visuals.px(22.0);
    for (i, (name, key)) in binds.iter().enumerate() {
        let y = area.min.y + visuals.px(6.0) + i as f32 * (row_h + visuals.px(2.0));
        if y + row_h > area.max.y - visuals.px(30.0) {
            break;
        }
        let row = Rect::from_min_size(
            pos2(area.min.x + visuals.px(8.0), y),
            vec2(area.width() - visuals.px(24.0), row_h),
        );
        let button_rect = Rect::from_min_size(
            pos2(row.max.x - visuals.px(120.0), row.min.y),
            vec2(visuals.px(120.0), row_h),
        );
        widgets::label(
            ui.painter(),
            name,
            row.min + vec2(visuals.px(2.0), visuals.px(5.0)),
            visuals,
            TEXT,
        );
        widgets::button(ui, button_rect, key, visuals, true);
    }
    let bar = Rect::from_min_size(
        pos2(area.max.x - visuals.px(8.0), area.min.y + visuals.px(6.0)),
        vec2(visuals.px(6.0), area.height() - visuals.px(36.0)),
    );
    widgets::scrollbar(ui.painter(), bar, 0.0, visuals);
    let by = area.max.y - visuals.px(26.0);
    if widgets::button(
        ui,
        Rect::from_center_size(
            pos2(area.center().x - visuals.px(100.0), by),
            vec2(visuals.px(180.0), visuals.px(20.0)),
        ),
        "Reset Keys",
        visuals,
        true,
    )
    .clicked()
    {
        state.time += 0.0;
    }
    if widgets::button(
        ui,
        Rect::from_center_size(
            pos2(area.center().x + visuals.px(100.0), by),
            vec2(visuals.px(180.0), visuals.px(20.0)),
        ),
        "Done",
        visuals,
        true,
    )
    .clicked()
    {
        state.back();
    }
}

fn language(
    ui: &mut Ui,
    rect: Rect,
    state: &mut MenuState,
    visuals: &Visuals,
    panorama: &mut Panorama,
) {
    full_menu_scaffold(ui, rect, state, visuals, panorama);
    let area = Rect::from_center_size(
        rect.center(),
        vec2(
            (visuals.px(360.0)).min(rect.width() - visuals.px(20.0)),
            (rect.height() - visuals.px(70.0)).max(visuals.px(120.0)),
        ),
    );
    let field = Rect::from_min_size(
        pos2(area.min.x + visuals.px(8.0), area.min.y + visuals.px(8.0)),
        vec2(area.width() - visuals.px(16.0), visuals.px(20.0)),
    );
    widgets::text_field(ui, field, "Search languages...", visuals);
    let list_top = field.max.y + visuals.px(8.0);
    widgets::list_background(
        ui.painter(),
        Rect::from_min_max(
            pos2(area.min.x + visuals.px(8.0), list_top),
            pos2(area.max.x - visuals.px(8.0), area.max.y - visuals.px(30.0)),
        ),
        visuals,
        false,
    );
    let row_h = visuals.px(18.0);
    for (i, entry) in LANGUAGES.iter().enumerate() {
        let y = list_top + visuals.px(4.0) + i as f32 * (row_h + visuals.px(2.0));
        let row = Rect::from_min_size(
            pos2(area.min.x + visuals.px(10.0), y),
            vec2(area.width() - visuals.px(28.0), row_h),
        );
        draw_language_row(ui, row, i, entry, state, visuals);
    }
    if widgets::button(
        ui,
        Rect::from_center_size(
            pos2(area.center().x, area.max.y - visuals.px(14.0)),
            vec2(visuals.px(200.0), visuals.px(20.0)),
        ),
        "Done",
        visuals,
        true,
    )
    .clicked()
    {
        state.back();
    }
}

fn draw_language_row(
    ui: &mut Ui,
    row: Rect,
    index: usize,
    entry: &LanguageEntry,
    state: &mut MenuState,
    visuals: &Visuals,
) {
    let response = ui.interact(row, ui.id().with(("lang", index)), Sense::click());
    if index == state.selected_language {
        ui.painter().rect_filled(
            row,
            egui::CornerRadius::ZERO,
            Color32::from_rgba_premultiplied(80, 100, 140, 120),
        );
    } else if response.hovered() {
        ui.painter().rect_filled(
            row,
            egui::CornerRadius::ZERO,
            Color32::from_rgba_premultiplied(70, 70, 80, 80),
        );
    }
    if response.clicked() {
        state.selected_language = index;
    }
    visuals.font.draw(
        ui.painter(),
        row.min + vec2(visuals.px(4.0), visuals.px(3.0)),
        entry.name,
        visuals.scale,
        if index == state.selected_language {
            GOLD
        } else {
            TEXT
        },
        true,
    );
}

fn accessibility(
    ui: &mut Ui,
    rect: Rect,
    state: &mut MenuState,
    visuals: &Visuals,
    panorama: &mut Panorama,
) {
    full_menu_scaffold(ui, rect, state, visuals, panorama);
    let area = Rect::from_center_size(
        rect.center(),
        vec2(
            (visuals.px(340.0)).min(rect.width() - visuals.px(20.0)),
            (rect.height() - visuals.px(70.0)).max(visuals.px(120.0)),
        ),
    );
    let mut y = area.min.y + visuals.px(16.0);
    let step = visuals.px(36.0);
    settings_row(ui, area, y, "Panorama Scroll Speed", visuals, |ui, r| {
        widgets::slider(ui, r, &mut state.settings.panorama_speed, visuals);
    });
    y += step;
    settings_row(ui, area, y, "High Contrast", visuals, |ui, r| {
        let check = Rect::from_min_size(r.min, vec2(visuals.px(20.0), visuals.px(20.0)));
        if widgets::checkbox(ui, check, state.settings.high_contrast, visuals).clicked() {
            state.settings.high_contrast = !state.settings.high_contrast;
        }
    });
    y += step;
    settings_row(ui, area, y, "Reduced Motion", visuals, |ui, r| {
        let check = Rect::from_min_size(r.min, vec2(visuals.px(20.0), visuals.px(20.0)));
        if widgets::checkbox(ui, check, state.settings.reduced_motion, visuals).clicked() {
            state.settings.reduced_motion = !state.settings.reduced_motion;
        }
    });
    y += step;
    settings_row(ui, area, y, "Text Background", visuals, |ui, r| {
        let check = Rect::from_min_size(r.min, vec2(visuals.px(20.0), visuals.px(20.0)));
        if widgets::checkbox(ui, check, state.settings.text_background, visuals).clicked() {
            state.settings.text_background = !state.settings.text_background;
        }
    });
    if widgets::button(
        ui,
        Rect::from_center_size(
            pos2(area.center().x, area.max.y - visuals.px(14.0)),
            vec2(visuals.px(200.0), visuals.px(20.0)),
        ),
        "Done",
        visuals,
        true,
    )
    .clicked()
    {
        state.back();
    }
}

fn pause(ui: &mut Ui, rect: Rect, state: &mut MenuState, visuals: &Visuals) {
    draw_fake_world(ui.painter(), rect, state.time);
    ui.painter().rect_filled(
        rect,
        egui::CornerRadius::ZERO,
        Color32::from_black_alpha(140),
    );
    let panel = Rect::from_center_size(
        rect.center(),
        vec2(
            (visuals.px(400.0)).min(rect.width() - visuals.px(16.0)),
            (visuals.px(260.0)).min(rect.height() - visuals.px(16.0)),
        ),
    );
    widgets::panel(ui.painter(), panel, visuals, true);
    header(ui.painter(), panel, state, visuals, true);
    visuals.font.draw_centered(
        ui.painter(),
        Rect::from_min_size(panel.min, vec2(panel.width(), visuals.px(40.0)))
            .translate(vec2(0.0, visuals.px(6.0))),
        "Game Paused",
        visuals.scale + 1,
        TEXT,
        true,
    );
    let bw = visuals.px(200.0).min(panel.width() - visuals.px(24.0));
    let bh = visuals.px(20.0);
    let gap = visuals.px(4.0);
    let start = panel.min.y + visuals.px(52.0);
    let buttons = column(panel, bw, bh, gap, start);
    let labels = [
        "Return to Game",
        "Options",
        "Save and Quit to Title",
        "Visual preview",
    ];
    for (r, text) in buttons.into_iter().zip(labels) {
        let enabled = text != "Visual preview";
        let response = widgets::button(ui, r, text, visuals, enabled);
        if response.clicked() {
            match text {
                "Options" => state.go(Screen::Options),
                "Save and Quit to Title" => {
                    state.reset_stack();
                    state.go(Screen::MainMenu);
                }
                _ => {}
            }
        }
    }
}

fn draw_fake_world(painter: &Painter, rect: Rect, time: f32) {
    let sky_top = Color32::from_rgb(86, 139, 190);
    let sky_bottom = Color32::from_rgb(160, 190, 210);
    let rows = 16;
    for i in 0..rows {
        let t = i as f32 / rows as f32;
        let y0 = rect.min.y + rect.height() * t;
        let y1 = rect.min.y + rect.height() * (t + 1.0 / rows as f32);
        let r = (sky_top.r() as f32 * (1.0 - t) + sky_bottom.r() as f32 * t) as u8;
        let g = (sky_top.g() as f32 * (1.0 - t) + sky_bottom.g() as f32 * t) as u8;
        let b = (sky_top.b() as f32 * (1.0 - t) + sky_bottom.b() as f32 * t) as u8;
        painter.rect_filled(
            Rect::from_min_max(pos2(rect.min.x, y0), pos2(rect.max.x, y1)),
            egui::CornerRadius::ZERO,
            Color32::from_rgb(r, g, b),
        );
    }
    let unit = (rect.width() / 32.0).max(8.0);
    let ground_y = rect.min.y + rect.height() * 0.62;
    for col in 0..40 {
        let x = rect.min.x + col as f32 * unit;
        let height = unit * (1.0 + ((col as usize * 7) % 5) as f32 * 0.35);
        painter.rect_filled(
            Rect::from_min_max(pos2(x, ground_y - height), pos2(x + unit, ground_y)),
            egui::CornerRadius::ZERO,
            Color32::from_rgb(72 + (col % 3) * 8, 118 + (col % 4) * 6, 66),
        );
        painter.rect_filled(
            Rect::from_min_max(pos2(x, ground_y), pos2(x + unit, rect.max.y)),
            egui::CornerRadius::ZERO,
            Color32::from_rgb(92, 70, 48),
        );
    }
    let sun_x = rect.min.x + rect.width() * (0.2 + (time * 0.004) % 0.6);
    painter.rect_filled(
        Rect::from_center_size(
            pos2(sun_x, rect.min.y + rect.height() * 0.2),
            vec2(unit, unit),
        ),
        egui::CornerRadius::ZERO,
        Color32::from_rgb(255, 244, 190),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palette_constants_are_opaque() {
        assert_eq!(TEXT.a(), 255);
        assert_eq!(DIM.a(), 255);
        assert_eq!(GOLD.a(), 255);
    }
}
