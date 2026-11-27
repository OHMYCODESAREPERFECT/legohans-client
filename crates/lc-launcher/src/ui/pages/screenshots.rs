//! Screenshots: thumbnails, a large viewer and an "open folder" button.

use egui::{
    Align2, Color32, CornerRadius, FontId, Id, Rect, RichText, Sense, Stroke, StrokeKind, pos2,
    vec2,
};

use super::{Ctx, caption};
use crate::model::screenshot::{ScreenshotEntry, scan};
use crate::services::images::ImageSource;
use crate::ui::icons::paint_spinner;
use crate::ui::toasts::ToastKind;
use crate::ui::widgets::{glass_panel, modal_frame, page_header, secondary_button, wide_button};
use crate::util::{format_bytes, now_secs, open_folder, relative_time};

const RESCAN_SECS: f64 = 5.0;
const MAX_SCREENSHOTS: usize = 400;
const THUMB: egui::Vec2 = vec2(236.0, 133.0);

/// Screenshots page state.
#[derive(Default)]
pub struct State {
    entries: Vec<ScreenshotEntry>,
    scanned_at: Option<f64>,
    viewing: Option<usize>,
}

fn scan_dirs(c: &Ctx<'_>) -> Vec<std::path::PathBuf> {
    let mut dirs = vec![c.state.data.dirs.screenshots_dir()];
    for p in &c.state.data.profiles.profiles {
        dirs.push(p.resolved_game_dir(&c.state.data.dirs).join("screenshots"));
    }
    dirs.sort();
    dirs.dedup();
    dirs
}

fn rescan(c: &mut Ctx<'_>) {
    let dirs = scan_dirs(c);
    c.ui.shots.entries = scan(&dirs, MAX_SCREENSHOTS);
    c.ui.shots.scanned_at = Some(c.now);
    if c.ui
        .shots
        .viewing
        .is_some_and(|i| i >= c.ui.shots.entries.len())
    {
        c.ui.shots.viewing = None;
    }
}

/// Draw the Screenshots page.
pub fn show(ui: &mut egui::Ui, c: &mut Ctx<'_>) {
    page_header(
        ui,
        c.theme,
        "Screenshots",
        "Screenshots taken by your installations.",
    );
    if c.ui
        .shots
        .scanned_at
        .is_none_or(|t| c.now - t > RESCAN_SECS)
    {
        rescan(c);
    }
    c.egui
        .request_repaint_after(std::time::Duration::from_secs_f64(RESCAN_SECS));

    let folder = c.state.data.dirs.screenshots_dir();
    ui.horizontal(|ui| {
        if wide_button(ui, c.theme, "Open screenshots folder", 0.0).clicked()
            && let Err(e) = open_folder(&folder)
        {
            c.toasts
                .push(ToastKind::Error, format!("Could not open the folder: {e}"));
        }
        if secondary_button(ui, c.theme, "Rescan").clicked() {
            rescan(c);
        }
        caption(
            ui,
            c.theme,
            &format!("{} screenshot(s)", c.ui.shots.entries.len()),
        );
    });
    ui.add_space(8.0);

    if c.ui.shots.entries.is_empty() {
        glass_panel(ui, c.theme, |ui| {
            ui.set_width(ui.available_width());
            ui.label(
                RichText::new("No screenshots yet")
                    .size(17.0)
                    .strong()
                    .color(c.theme.text),
            );
            caption(
                ui,
                c.theme,
                "PNG screenshots placed in these folders show up here:",
            );
            for d in scan_dirs(c) {
                caption(ui, c.theme, &format!("  {}", d.display()));
            }
        });
        return;
    }

    let entries = c.ui.shots.entries.clone();
    let now = now_secs();
    let mut open = None;
    egui::ScrollArea::vertical()
        .id_salt("shots_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing = vec2(14.0, 14.0);
            ui.horizontal_wrapped(|ui| {
                for (i, e) in entries.iter().enumerate() {
                    let size = vec2(THUMB.x, THUMB.y + 44.0);
                    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
                    let hover = ui.ctx().animate_bool_with_time(
                        resp.id,
                        resp.hovered(),
                        c.theme.anim(0.15),
                    );
                    let img_rect = Rect::from_min_size(rect.min, THUMB);
                    let painter = ui.painter();
                    painter.rect_filled(rect, CornerRadius::same(12), c.theme.glass);
                    painter.rect_filled(
                        img_rect,
                        CornerRadius {
                            nw: 12,
                            ne: 12,
                            sw: 0,
                            se: 0,
                        },
                        Color32::from_black_alpha(120),
                    );
                    if ui.is_rect_visible(rect) {
                        let key = format!("shot:{}", e.path.display());
                        c.textures
                            .request(&key, ImageSource::PngFile(e.path.clone()), 480);
                        match c.textures.get(&key) {
                            Some(tex) => {
                                ui.painter().image(
                                    tex.id(),
                                    img_rect.shrink(1.0),
                                    Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                                    Color32::WHITE,
                                );
                            }
                            None if c.textures.has_failed(&key) => {
                                ui.painter().text(
                                    img_rect.center(),
                                    Align2::CENTER_CENTER,
                                    "Cannot preview",
                                    FontId::proportional(13.0),
                                    c.theme.text_faint,
                                );
                            }
                            None => paint_spinner(
                                ui.painter(),
                                img_rect.center(),
                                10.0,
                                c.now,
                                c.theme.accent_hi,
                            ),
                        }
                    }
                    let painter = ui.painter();
                    painter.rect_stroke(
                        rect,
                        CornerRadius::same(12),
                        Stroke::new(
                            1.0 + 0.5 * hover,
                            c.theme.border.lerp_to_gamma(c.theme.accent, hover),
                        ),
                        StrokeKind::Inside,
                    );
                    painter.text(
                        pos2(rect.left() + 12.0, img_rect.bottom() + 14.0),
                        Align2::LEFT_CENTER,
                        truncate(&e.name, 30),
                        FontId::proportional(13.5),
                        c.theme.text,
                    );
                    painter.text(
                        pos2(rect.left() + 12.0, img_rect.bottom() + 31.0),
                        Align2::LEFT_CENTER,
                        relative_time(now, e.modified),
                        FontId::proportional(12.0),
                        c.theme.text_faint,
                    );
                    if resp
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .clicked()
                    {
                        open = Some(i);
                    }
                }
            });
        });
    if open.is_some() {
        c.ui.shots.viewing = open;
    }
    viewer(c);
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let cut: String = s.chars().take(max - 1).collect();
        format!("{cut}…")
    }
}

fn viewer(c: &mut Ctx<'_>) {
    let Some(index) = c.ui.shots.viewing else {
        return;
    };
    let Some(entry) = c.ui.shots.entries.get(index).cloned() else {
        c.ui.shots.viewing = None;
        return;
    };
    let key = format!("shotbig:{}", entry.path.display());
    c.textures
        .request(&key, ImageSource::PngFile(entry.path.clone()), 1600);

    let theme = c.theme;
    let screen = c.egui.content_rect();
    let max_img = vec2(
        (screen.width() * 0.78).min(1100.0),
        (screen.height() * 0.64).min(700.0),
    );
    let total = c.ui.shots.entries.len();
    let mut step: i32 = 0;
    let mut close = false;
    let mut open_dir = false;
    let modal = egui::Modal::new(Id::new("shot_viewer"))
        .frame(modal_frame(theme))
        .show(c.egui, |ui| {
            ui.set_width(max_img.x);
            ui.label(
                RichText::new(&entry.name)
                    .size(18.0)
                    .strong()
                    .color(theme.text),
            );
            ui.label(
                RichText::new(format!(
                    "{}  ·  {}  ·  {} of {}",
                    relative_time(now_secs(), entry.modified),
                    format_bytes(entry.size),
                    index + 1,
                    total
                ))
                .size(13.0)
                .color(theme.text_dim),
            );
            ui.add_space(6.0);
            let (rect, _) = ui.allocate_exact_size(max_img, Sense::hover());
            ui.painter()
                .rect_filled(rect, CornerRadius::same(10), Color32::from_black_alpha(140));
            match c.textures.get(&key) {
                Some(tex) => {
                    let tex_size = tex.size_vec2();
                    let scale = (max_img.x / tex_size.x)
                        .min(max_img.y / tex_size.y)
                        .clamp(0.01, 1.0);
                    let fit = Rect::from_center_size(rect.center(), tex_size * scale);
                    ui.painter().image(
                        tex.id(),
                        fit,
                        Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                        Color32::WHITE,
                    );
                }
                None if c.textures.has_failed(&key) => {
                    ui.painter().text(
                        rect.center(),
                        Align2::CENTER_CENTER,
                        "This image cannot be previewed",
                        FontId::proportional(16.0),
                        theme.text_dim,
                    );
                }
                None => paint_spinner(ui.painter(), rect.center(), 14.0, c.now, theme.accent_hi),
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.add_enabled_ui(index > 0, |ui| {
                    if secondary_button(ui, theme, "Previous").clicked() {
                        step = -1;
                    }
                });
                ui.add_enabled_ui(index + 1 < total, |ui| {
                    if secondary_button(ui, theme, "Next").clicked() {
                        step = 1;
                    }
                });
                if secondary_button(ui, theme, "Open folder").clicked() {
                    open_dir = true;
                }
                if secondary_button(ui, theme, "Close").clicked() {
                    close = true;
                }
            });
        });
    if modal.should_close() {
        close = true;
    }
    if open_dir
        && let Some(parent) = entry.path.parent()
        && let Err(e) = open_folder(parent)
    {
        c.toasts
            .push(ToastKind::Error, format!("Could not open the folder: {e}"));
    }
    if step != 0 {
        let next = (index as i32 + step).clamp(0, total as i32 - 1) as usize;
        c.ui.shots.viewing = Some(next);
    }
    if close {
        c.ui.shots.viewing = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncation() {
        assert_eq!(truncate("abc", 5), "abc");
        assert_eq!(truncate("abcdefgh", 4), "abc…");
    }
}
