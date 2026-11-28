//! Skin editor: imports Minecraft textures and renders a real 3D preview with egui meshes.

use std::f32::consts::TAU;
use std::path::{Path, PathBuf};

use egui::epaint::{Mesh, Vertex};
use egui::{Color32, CursorIcon, Pos2, Rect, RichText, Sense, Shape, TextureId, pos2, vec2};

use super::{Ctx, caption};
use crate::model::settings::SkinModel;
use crate::model::skin::{self, TextureKind};
use crate::services::images::png_dimensions;
use crate::ui::toasts::ToastKind;
use crate::ui::widgets::{glass_panel, page_header, secondary_button, toggle};

const SKIN_KEY: &str = "custom-skin";
const CAPE_KEY: &str = "custom-cape";
const ELYTRA_KEY: &str = "custom-elytra";
const WHITE_KEY: &str = "skin-white";
const DEFAULT_YAW: f32 = 0.45;
const DEFAULT_PITCH: f32 = 0.08;

/// Skin editor page state.
pub struct State {
    yaw: f32,
    pitch: f32,
    show_outer: bool,
    show_cape: bool,
    show_elytra: bool,
    legacy_skin: bool,
    preview_rect: Option<Rect>,
    skin_drop_rect: Option<Rect>,
    cape_drop_rect: Option<Rect>,
    elytra_drop_rect: Option<Rect>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            yaw: DEFAULT_YAW,
            pitch: DEFAULT_PITCH,
            show_outer: true,
            show_cape: true,
            show_elytra: true,
            legacy_skin: false,
            preview_rect: None,
            skin_drop_rect: None,
            cape_drop_rect: None,
            elytra_drop_rect: None,
        }
    }
}

/// Draw the Skin Editor page.
pub fn show(ui: &mut egui::Ui, c: &mut Ctx<'_>) {
    handle_drops(ui, c);
    ensure_textures(c);
    page_header(
        ui,
        c.theme,
        "Skin Editor",
        "Import skins, capes and elytra, then drag the model to inspect every side.",
    );
    ui.add_space(4.0);
    ui.horizontal_top(|ui| {
        ui.vertical(|ui| {
            ui.set_min_width(270.0);
            ui.set_max_width(320.0);
            controls(ui, c);
        });
        ui.separator();
        ui.vertical(|ui| {
            ui.set_min_width(260.0);
            preview(ui, c);
        });
    });
}

fn handle_drops(ui: &mut egui::Ui, c: &mut Ctx<'_>) {
    let dropped: Vec<PathBuf> = ui.input(|i| {
        i.raw
            .dropped_files
            .iter()
            .filter_map(|f| f.path.clone())
            .collect()
    });
    if dropped.is_empty() {
        return;
    }
    let pointer = ui.input(|i| i.pointer.latest_pos());
    for path in dropped {
        let kind = pointer
            .and_then(|p| drop_kind(&c.ui.skin_editor, p))
            .unwrap_or_else(|| infer_kind(&path));
        import(c, &path, kind);
    }
}

fn drop_kind(state: &State, pos: Pos2) -> Option<TextureKind> {
    if state.cape_drop_rect.is_some_and(|r| r.contains(pos)) {
        Some(TextureKind::Cape)
    } else if state.elytra_drop_rect.is_some_and(|r| r.contains(pos)) {
        Some(TextureKind::Elytra)
    } else if state.skin_drop_rect.is_some_and(|r| r.contains(pos))
        || state.preview_rect.is_some_and(|r| r.contains(pos))
    {
        Some(TextureKind::Skin)
    } else {
        None
    }
}

fn infer_kind(path: &Path) -> TextureKind {
    let bytes = std::fs::read(path).unwrap_or_default();
    let dims = png_dimensions(&bytes).ok();
    match dims {
        Some((64, 64)) => TextureKind::Skin,
        Some((64, 32)) => {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            if name.contains("elytra") || name.contains("wing") {
                TextureKind::Elytra
            } else {
                TextureKind::Cape
            }
        }
        _ => TextureKind::Skin,
    }
}

fn texture_key(kind: TextureKind) -> &'static str {
    match kind {
        TextureKind::Skin => SKIN_KEY,
        TextureKind::Cape => CAPE_KEY,
        TextureKind::Elytra => ELYTRA_KEY,
    }
}

fn texture_path(c: &Ctx<'_>, kind: TextureKind) -> PathBuf {
    match kind {
        TextureKind::Skin => c.state.data.dirs.skin_file(),
        TextureKind::Cape => c.state.data.dirs.cape_file(),
        TextureKind::Elytra => c.state.data.dirs.elytra_file(),
    }
}

fn ensure_texture(c: &mut Ctx<'_>, kind: TextureKind) {
    let key = texture_key(kind);
    if c.textures.get(key).is_some() || c.textures.has_failed(key) {
        return;
    }
    let path = texture_path(c, kind);
    if !skin::texture_exists(&path, kind) {
        return;
    }
    let Ok(bytes) = std::fs::read(&path) else {
        return;
    };
    if kind == TextureKind::Skin
        && let Ok(info) = skin::validate_skin(&bytes)
    {
        c.ui.skin_editor.legacy_skin = info.legacy;
    }
    c.textures.load_bytes_nearest(key, &bytes, 64);
}

fn ensure_textures(c: &mut Ctx<'_>) {
    ensure_texture(c, TextureKind::Skin);
    ensure_texture(c, TextureKind::Cape);
    ensure_texture(c, TextureKind::Elytra);
    if c.textures.get(WHITE_KEY).is_none() {
        c.textures.load_bytes_nearest(WHITE_KEY, &solid_png(), 1);
    }
}

fn import(c: &mut Ctx<'_>, source: &Path, kind: TextureKind) {
    let target = texture_path(c, kind);
    match skin::import_texture(source, &target, kind) {
        Ok(info) => {
            c.textures.evict(texture_key(kind));
            if kind == TextureKind::Skin {
                c.ui.skin_editor.legacy_skin = info.legacy;
            }
            c.toasts.push(
                ToastKind::Success,
                format!(
                    "Imported {} ({}x{}).",
                    kind.label(),
                    info.width,
                    info.height
                ),
            );
        }
        Err(e) => c.toasts.push(ToastKind::Error, e),
    }
}

fn remove(c: &mut Ctx<'_>, kind: TextureKind) {
    let path = texture_path(c, kind);
    match std::fs::remove_file(&path) {
        Ok(()) => {
            c.textures.evict(texture_key(kind));
            c.toasts
                .push(ToastKind::Info, format!("Removed the {}.", kind.label()));
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => c.toasts.push(
            ToastKind::Error,
            format!("Could not remove {}: {e}", path.display()),
        ),
    }
}

fn controls(ui: &mut egui::Ui, c: &mut Ctx<'_>) {
    let theme = c.theme.clone();
    glass_panel(ui, &theme, |ui| {
        ui.label(
            RichText::new("Player model")
                .size(16.0)
                .strong()
                .color(theme.text),
        );
        let model = c.state.data.settings.skin_model;
        egui::ComboBox::from_id_salt("skin_model")
            .selected_text(model.name())
            .show_ui(ui, |ui| {
                for option in SkinModel::ALL {
                    ui.selectable_value(
                        &mut c.state.data.settings.skin_model,
                        option,
                        option.name(),
                    );
                }
            });
        caption(
            ui,
            &theme,
            "Slim uses Alex's 3-pixel arms; Classic uses Steve's 4-pixel arms.",
        );
        ui.add_space(6.0);
        toggle(ui, &theme, &mut c.ui.skin_editor.show_outer);
        ui.label(
            RichText::new("Outer layers (hat, jacket, sleeves)")
                .size(13.0)
                .color(theme.text_dim),
        );
        ui.add_space(10.0);
        texture_control(ui, c, TextureKind::Skin);
    });

    ui.add_space(10.0);
    glass_panel(ui, &theme, |ui| {
        ui.label(
            RichText::new("Cape & elytra")
                .size(16.0)
                .strong()
                .color(theme.text),
        );
        caption(
            ui,
            &theme,
            "Both textures are 64x32. Drop a file on its row.",
        );
        ui.add_space(6.0);
        texture_control(ui, c, TextureKind::Cape);
        ui.add_space(8.0);
        texture_control(ui, c, TextureKind::Elytra);
        ui.add_space(8.0);
        toggle(ui, &theme, &mut c.ui.skin_editor.show_cape);
        ui.label(RichText::new("Show cape").size(13.0).color(theme.text_dim));
        toggle(ui, &theme, &mut c.ui.skin_editor.show_elytra);
        ui.label(
            RichText::new("Show elytra (hides the cape)")
                .size(13.0)
                .color(theme.text_dim),
        );
    });
}

fn texture_control(ui: &mut egui::Ui, c: &mut Ctx<'_>, kind: TextureKind) {
    let theme = c.theme.clone();
    let key = texture_key(kind);
    let label = match kind {
        TextureKind::Skin => "Skin",
        TextureKind::Cape => "Cape",
        TextureKind::Elytra => "Elytra",
    };
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).size(13.5).strong().color(theme.text));
        if secondary_button(ui, &theme, "Import…").clicked()
            && let Some(file) = crate::util::pick_png(&format!("Choose a {label} PNG"))
        {
            import(c, &file, kind);
        }
        if c.textures.get(key).is_some() && secondary_button(ui, &theme, "Remove").clicked() {
            remove(c, kind);
        }
    });

    let (rect, _) = ui.allocate_exact_size(vec2(250.0, 36.0), Sense::hover());
    ui.painter().rect_filled(
        rect,
        egui::CornerRadius::same(8),
        theme.accent_alpha(if c.textures.get(key).is_some() {
            42
        } else {
            16
        }),
    );
    ui.painter().rect_stroke(
        rect,
        egui::CornerRadius::same(8),
        egui::Stroke::new(1.0, theme.border),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        if c.textures.get(key).is_some() {
            format!("{label} ready — drop to replace")
        } else {
            format!("Drop a {label} PNG here")
        },
        egui::FontId::proportional(12.5),
        theme.text_dim,
    );
    match kind {
        TextureKind::Skin => c.ui.skin_editor.skin_drop_rect = Some(rect),
        TextureKind::Cape => c.ui.skin_editor.cape_drop_rect = Some(rect),
        TextureKind::Elytra => c.ui.skin_editor.elytra_drop_rect = Some(rect),
    }

    if let Some(tex) = c.textures.get(key) {
        let (thumb, _) = ui.allocate_exact_size(vec2(48.0, 48.0), Sense::hover());
        ui.painter().image(
            tex.id(),
            thumb,
            egui::Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            Color32::WHITE,
        );
    }
}

fn preview(ui: &mut egui::Ui, c: &mut Ctx<'_>) {
    let theme = c.theme.clone();
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("3D preview")
                .size(16.0)
                .strong()
                .color(theme.text),
        );
        if secondary_button(ui, &theme, "Reset view").clicked() {
            c.ui.skin_editor.yaw = DEFAULT_YAW;
            c.ui.skin_editor.pitch = DEFAULT_PITCH;
        }
    });
    caption(ui, &theme, "Drag anywhere on the preview to rotate 360°.");

    let (rect, resp) = ui.allocate_exact_size(
        vec2(ui.available_width().max(240.0), 480.0),
        Sense::click_and_drag(),
    );
    c.ui.skin_editor.preview_rect = Some(rect);
    if resp.dragged() {
        let delta = ui.input(|i| i.pointer.delta());
        c.ui.skin_editor.yaw += delta.x * 0.012;
        c.ui.skin_editor.pitch = (c.ui.skin_editor.pitch - delta.y * 0.01).clamp(-0.65, 0.65);
        ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
    } else {
        resp.on_hover_cursor(CursorIcon::Grab);
    }

    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, egui::CornerRadius::same(16), theme.accent_alpha(18));
    painter.rect_stroke(
        rect,
        egui::CornerRadius::same(16),
        egui::Stroke::new(1.0, theme.border),
        egui::StrokeKind::Inside,
    );
    let origin = rect.center();
    let scale = (rect.height() * 0.82 / 32.0).min(rect.width() / 26.0);
    let yaw = c.ui.skin_editor.yaw.rem_euclid(TAU);
    let pitch = c.ui.skin_editor.pitch;
    let model = c.state.data.settings.skin_model;
    let white = c
        .textures
        .get(WHITE_KEY)
        .map(TextureHandleExt::texture_id)
        .unwrap_or_default();

    // Cape and elytra are attached behind the shoulders. Their draw order follows the camera so
    // they are hidden by the body from the front and occlude it when viewed from the back.
    let mut back_mesh = if c.ui.skin_editor.show_elytra && c.textures.get(ELYTRA_KEY).is_some() {
        let tex = c.textures.get(ELYTRA_KEY).unwrap();
        Some(build_box_mesh(
            &elytra_boxes(),
            32.0,
            tex.id(),
            yaw,
            pitch,
            origin,
            scale,
        ))
    } else if c.ui.skin_editor.show_cape && c.textures.get(CAPE_KEY).is_some() {
        let tex = c.textures.get(CAPE_KEY).unwrap();
        Some(build_box_mesh(
            &cape_boxes(),
            32.0,
            tex.id(),
            yaw,
            pitch,
            origin,
            scale,
        ))
    } else {
        None
    };
    let back_in_front = back_equipment_in_front(yaw);
    if !back_in_front && let Some(mesh) = back_mesh.take() {
        painter.add(Shape::mesh(mesh));
    }

    let skin_tex = c.textures.get(SKIN_KEY);
    let tex_id = skin_tex.map(|t| t.id()).unwrap_or(white);
    let base = player_boxes(model, false, c.ui.skin_editor.legacy_skin);
    painter.add(Shape::mesh(build_box_mesh(
        &base,
        skin_texture_height(c.ui.skin_editor.legacy_skin),
        tex_id,
        yaw,
        pitch,
        origin,
        scale,
    )));
    if c.ui.skin_editor.show_outer && !c.ui.skin_editor.legacy_skin {
        let outer = player_boxes(model, true, false);
        painter.add(Shape::mesh(build_box_mesh(
            &outer, 64.0, tex_id, yaw, pitch, origin, scale,
        )));
    }

    if back_in_front && let Some(mesh) = back_mesh {
        painter.add(Shape::mesh(mesh));
    }

    if skin_tex.is_none() {
        painter.text(
            rect.center() + vec2(0.0, rect.height() * 0.36),
            egui::Align2::CENTER_CENTER,
            "Import a skin to see its colors",
            egui::FontId::proportional(13.0),
            theme.text_dim,
        );
    }
}

// A tiny extension trait keeps `TextureHandle::id()` readable in the chain above.
trait TextureHandleExt {
    fn texture_id(&self) -> TextureId;
}

impl TextureHandleExt for egui::TextureHandle {
    fn texture_id(&self) -> TextureId {
        self.id()
    }
}

// ---------------------------------------------------------------------------
// Geometry and projection

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct V3 {
    x: f32,
    y: f32,
    z: f32,
}

impl V3 {
    fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    fn add(self, rhs: Self) -> Self {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }

    fn sub(self, rhs: Self) -> Self {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

/// A rigid rotation around a model pivot. Angles are applied roll, yaw, then pitch to match
/// three.js' default Euler order for the reference model.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Transform {
    pivot: V3,
    yaw: f32,
    pitch: f32,
    roll: f32,
}

impl Transform {
    fn direction(self, v: V3) -> V3 {
        rotate_axes(v, self.yaw, self.pitch, self.roll)
    }

    fn apply(self, v: V3) -> V3 {
        self.direction(v.sub(self.pivot)).add(self.pivot)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Uv {
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
}

fn uv(x0: f32, y0: f32, x1: f32, y1: f32) -> Uv {
    Uv { x0, y0, x1, y1 }
}

#[derive(Clone, Copy, Debug)]
struct BoxUv {
    top: Uv,
    bottom: Uv,
    front: Uv,
    back: Uv,
    right: Uv,
    left: Uv,
}

#[derive(Clone, Copy)]
struct BoxSpec {
    min: V3,
    max: V3,
    uv: BoxUv,
    transform: Transform,
}

#[derive(Clone, Copy)]
struct Face {
    verts: [V3; 4],
    uv: Uv,
    normal: V3,
    shade: f32,
}

fn rotate_axes(v: V3, yaw: f32, pitch: f32, roll: f32) -> V3 {
    let (cr, sr) = (roll.cos(), roll.sin());
    let x0 = v.x * cr - v.y * sr;
    let y0 = v.x * sr + v.y * cr;
    let (cy, sy) = (yaw.cos(), yaw.sin());
    let x1 = x0 * cy + v.z * sy;
    let z1 = -x0 * sy + v.z * cy;
    let (cp, sp) = (pitch.cos(), pitch.sin());
    let y2 = y0 * cp - z1 * sp;
    let z2 = y0 * sp + z1 * cp;
    V3::new(x1, y2, z2)
}

fn rotate(v: V3, yaw: f32, pitch: f32) -> V3 {
    rotate_axes(v, yaw, pitch, 0.0)
}

fn project(v: V3, center: V3, yaw: f32, pitch: f32, origin: Pos2, scale: f32) -> (Pos2, f32) {
    let r = rotate(
        V3::new(v.x - center.x, v.y - center.y, v.z - center.z),
        yaw,
        pitch,
    );
    (pos2(origin.x + r.x * scale, origin.y - r.y * scale), r.z)
}

fn uv_corners(region: Uv, tex_h: f32) -> [Pos2; 4] {
    [
        pos2(region.x0 / 64.0, region.y0 / tex_h),
        pos2(region.x0 / 64.0, region.y1 / tex_h),
        pos2(region.x1 / 64.0, region.y1 / tex_h),
        pos2(region.x1 / 64.0, region.y0 / tex_h),
    ]
}

fn box_faces(spec: BoxSpec) -> [Face; 6] {
    let transform = spec.transform;
    let V3 {
        x: x0,
        y: y0,
        z: z0,
    } = spec.min;
    let V3 {
        x: x1,
        y: y1,
        z: z1,
    } = spec.max;
    let mut faces: [Face; 6] = [
        Face {
            verts: [
                V3::new(x0, y1, z1),
                V3::new(x0, y0, z1),
                V3::new(x1, y0, z1),
                V3::new(x1, y1, z1),
            ],
            uv: spec.uv.front,
            normal: V3::new(0.0, 0.0, 1.0),
            shade: 1.0,
        },
        Face {
            verts: [
                V3::new(x1, y1, z0),
                V3::new(x1, y0, z0),
                V3::new(x0, y0, z0),
                V3::new(x0, y1, z0),
            ],
            uv: spec.uv.back,
            normal: V3::new(0.0, 0.0, -1.0),
            shade: 0.86,
        },
        Face {
            verts: [
                V3::new(x1, y1, z1),
                V3::new(x1, y0, z1),
                V3::new(x1, y0, z0),
                V3::new(x1, y1, z0),
            ],
            uv: spec.uv.right,
            normal: V3::new(1.0, 0.0, 0.0),
            shade: 0.78,
        },
        Face {
            verts: [
                V3::new(x0, y1, z0),
                V3::new(x0, y0, z0),
                V3::new(x0, y0, z1),
                V3::new(x0, y1, z1),
            ],
            uv: spec.uv.left,
            normal: V3::new(-1.0, 0.0, 0.0),
            shade: 0.78,
        },
        Face {
            verts: [
                V3::new(x0, y1, z0),
                V3::new(x0, y1, z1),
                V3::new(x1, y1, z1),
                V3::new(x1, y1, z0),
            ],
            uv: spec.uv.top,
            normal: V3::new(0.0, 1.0, 0.0),
            shade: 0.94,
        },
        Face {
            verts: [
                V3::new(x0, y0, z1),
                V3::new(x0, y0, z0),
                V3::new(x1, y0, z0),
                V3::new(x1, y0, z1),
            ],
            uv: spec.uv.bottom,
            normal: V3::new(0.0, -1.0, 0.0),
            shade: 0.62,
        },
    ];
    for face in &mut faces {
        face.verts = face.verts.map(|v| transform.apply(v));
        face.normal = transform.direction(face.normal);
    }
    faces
}

struct Triangle {
    points: [ProjectedPoint; 3],
    depth: f32,
}

#[derive(Clone, Copy, Debug)]
struct ProjectedPoint {
    pos: Pos2,
    uv: Pos2,
    color: Color32,
}

#[derive(Clone, Copy)]
struct Projection {
    center: V3,
    yaw: f32,
    pitch: f32,
    origin: Pos2,
    scale: f32,
}

impl Projection {
    fn player(yaw: f32, pitch: f32, origin: Pos2, scale: f32) -> Self {
        Self {
            center: V3::new(0.0, 16.0, 0.0),
            yaw,
            pitch,
            origin,
            scale,
        }
    }
}

/// Whether back equipment is closer to the camera than the player's torso.
fn back_equipment_in_front(yaw: f32) -> bool {
    rotate(V3::new(0.0, 0.0, -2.5), yaw, 0.0).z > 0.01
}

fn push_face(triangles: &mut Vec<Triangle>, face: Face, tex_h: f32, view: Projection) {
    if rotate(face.normal, view.yaw, view.pitch).z <= 0.0 {
        return;
    }
    let mut projected = Vec::with_capacity(4);
    let mut depth = 0.0;
    for (vertex, tex) in face.verts.iter().zip(uv_corners(face.uv, tex_h)) {
        let (pos, z) = project(
            *vertex,
            view.center,
            view.yaw,
            view.pitch,
            view.origin,
            view.scale,
        );
        depth += z;
        let shade = (255.0 * face.shade).round().clamp(0.0, 255.0) as u8;
        projected.push(ProjectedPoint {
            pos,
            uv: tex,
            color: Color32::from_rgb(shade, shade, shade),
        });
    }
    let p: [ProjectedPoint; 4] = projected.try_into().unwrap();
    triangles.push(Triangle {
        points: [p[0], p[1], p[2]],
        depth: depth / 4.0,
    });
    triangles.push(Triangle {
        points: [p[0], p[2], p[3]],
        depth: depth / 4.0,
    });
}

fn triangles_to_mesh(triangles: Vec<Triangle>, texture_id: TextureId) -> Mesh {
    let mut triangles = triangles;
    triangles.sort_by(|a, b| a.depth.total_cmp(&b.depth));
    let mut mesh = Mesh {
        texture_id,
        ..Mesh::default()
    };
    for triangle in triangles {
        for point in triangle.points {
            mesh.vertices.push(Vertex {
                pos: point.pos,
                uv: point.uv,
                color: point.color,
            });
        }
        let n = mesh.vertices.len() as u32;
        mesh.indices.extend([n - 3, n - 2, n - 1]);
    }
    mesh
}

fn build_box_mesh(
    boxes: &[BoxSpec],
    tex_h: f32,
    texture_id: TextureId,
    yaw: f32,
    pitch: f32,
    origin: Pos2,
    scale: f32,
) -> Mesh {
    let view = Projection::player(yaw, pitch, origin, scale);
    let mut triangles = Vec::new();
    for &spec in boxes {
        for face in box_faces(spec) {
            push_face(&mut triangles, face, tex_h, view);
        }
    }
    triangles_to_mesh(triangles, texture_id)
}

// ---------------------------------------------------------------------------
// Standard Minecraft texture coordinates

fn head_uv(outer: bool) -> BoxUv {
    if outer {
        BoxUv {
            top: uv(40.0, 0.0, 48.0, 8.0),
            bottom: uv(48.0, 0.0, 56.0, 8.0),
            front: uv(40.0, 8.0, 48.0, 16.0),
            back: uv(56.0, 8.0, 64.0, 16.0),
            right: uv(48.0, 8.0, 56.0, 16.0),
            left: uv(32.0, 8.0, 40.0, 16.0),
        }
    } else {
        BoxUv {
            top: uv(8.0, 0.0, 16.0, 8.0),
            bottom: uv(16.0, 0.0, 24.0, 8.0),
            front: uv(8.0, 8.0, 16.0, 16.0),
            back: uv(24.0, 8.0, 32.0, 16.0),
            right: uv(16.0, 8.0, 24.0, 16.0),
            left: uv(0.0, 8.0, 8.0, 16.0),
        }
    }
}

fn body_uv(outer: bool) -> BoxUv {
    let y = if outer { 32.0 } else { 16.0 };
    let side_y = if outer { 36.0 } else { 20.0 };
    BoxUv {
        top: uv(20.0, y, 28.0, y + 4.0),
        bottom: uv(28.0, y, 36.0, y + 4.0),
        front: uv(20.0, side_y, 28.0, side_y + 12.0),
        back: uv(32.0, side_y, 40.0, side_y + 12.0),
        right: uv(28.0, side_y, 32.0, side_y + 12.0),
        left: uv(16.0, side_y, 20.0, side_y + 12.0),
    }
}

fn right_arm_uv(model: SkinModel, outer: bool) -> BoxUv {
    let w = model.arm_width();
    let ty = if outer { 32.0 } else { 16.0 };
    let sy = if outer { 36.0 } else { 20.0 };
    // A slim arm is three pixels wide inside a four-pixel template column; the unused column is
    // the last one of each block, so every face keeps a fixed four-pixel stride.
    BoxUv {
        top: uv(44.0, ty, 44.0 + w, ty + 4.0),
        bottom: uv(48.0, ty, 48.0 + w, ty + 4.0),
        front: uv(44.0, sy, 44.0 + w, sy + 12.0),
        back: uv(52.0, sy, 52.0 + w, sy + 12.0),
        right: uv(48.0, sy, 48.0 + w, sy + 12.0),
        left: uv(40.0, sy, 40.0 + w, sy + 12.0),
    }
}

fn left_arm_uv(model: SkinModel, outer: bool) -> BoxUv {
    let w = model.arm_width();
    let (top_x, side_x) = if outer { (52.0, 48.0) } else { (36.0, 32.0) };
    BoxUv {
        top: uv(top_x, 48.0, top_x + w, 52.0),
        bottom: uv(top_x + 4.0, 48.0, top_x + 4.0 + w, 52.0),
        front: uv(side_x + 4.0, 52.0, side_x + 4.0 + w, 64.0),
        back: uv(side_x + 12.0, 52.0, side_x + 12.0 + w, 64.0),
        right: uv(side_x + 8.0, 52.0, side_x + 8.0 + w, 64.0),
        left: uv(side_x, 52.0, side_x + w, 64.0),
    }
}

fn right_leg_uv(outer: bool) -> BoxUv {
    let y = if outer { 32.0 } else { 16.0 };
    let sy = if outer { 36.0 } else { 20.0 };
    BoxUv {
        top: uv(4.0, y, 8.0, y + 4.0),
        bottom: uv(8.0, y, 12.0, y + 4.0),
        front: uv(4.0, sy, 8.0, sy + 12.0),
        back: uv(12.0, sy, 16.0, sy + 12.0),
        right: uv(8.0, sy, 12.0, sy + 12.0),
        left: uv(0.0, sy, 4.0, sy + 12.0),
    }
}

fn left_leg_uv(outer: bool) -> BoxUv {
    let (top_x, side_x) = if outer { (4.0, 0.0) } else { (20.0, 16.0) };
    BoxUv {
        top: uv(top_x, 48.0, top_x + 4.0, 52.0),
        bottom: uv(top_x + 4.0, 48.0, top_x + 8.0, 52.0),
        front: uv(side_x + 4.0, 52.0, side_x + 8.0, 64.0),
        back: uv(side_x + 12.0, 52.0, side_x + 16.0, 64.0),
        right: uv(side_x + 8.0, 52.0, side_x + 12.0, 64.0),
        left: uv(side_x, 52.0, side_x + 4.0, 64.0),
    }
}

/// Mirror the right-limb UVs onto a left limb: every face is flipped horizontally and the inner
/// and outer side faces change places.
fn mirrored_legacy_limb(uvs: BoxUv) -> BoxUv {
    let flip = |u: Uv| uv(u.x1, u.y0, u.x0, u.y1);
    BoxUv {
        top: flip(uvs.top),
        bottom: flip(uvs.bottom),
        front: flip(uvs.front),
        back: flip(uvs.back),
        right: flip(uvs.left),
        left: flip(uvs.right),
    }
}

fn player_boxes(model: SkinModel, outer: bool, legacy: bool) -> Vec<BoxSpec> {
    if legacy && outer {
        return Vec::new();
    }
    let box_spec = |min: V3, max: V3, uvs: BoxUv, expand: f32| BoxSpec {
        min: V3::new(min.x - expand, min.y - expand, min.z - expand),
        max: V3::new(max.x + expand, max.y + expand, max.z + expand),
        uv: uvs,
        transform: Transform::default(),
    };
    // Vanilla overlays are half a pixel wider on the head and a quarter of a pixel wider
    // everywhere else.
    let head_expand = if outer { 0.5 } else { 0.0 };
    let part_expand = if outer { 0.25 } else { 0.0 };
    // Legacy 64x32 skins have no separate left limb or lower body rows and no slim variant.
    // Minecraft mirrors the right arm and leg regions for both sides, so we do the same.
    let model = if legacy { SkinModel::Classic } else { model };
    let arm_w = model.arm_width();
    let (right_arm, left_arm, right_leg, left_leg) = if legacy {
        let arm = right_arm_uv(model, false);
        let leg = right_leg_uv(false);
        (
            arm,
            mirrored_legacy_limb(arm),
            leg,
            mirrored_legacy_limb(leg),
        )
    } else {
        (
            right_arm_uv(model, outer),
            left_arm_uv(model, outer),
            right_leg_uv(outer),
            left_leg_uv(outer),
        )
    };
    vec![
        box_spec(
            V3::new(-4.0, 24.0, -4.0),
            V3::new(4.0, 32.0, 4.0),
            head_uv(outer),
            head_expand,
        ),
        box_spec(
            V3::new(-4.0, 12.0, -2.0),
            V3::new(4.0, 24.0, 2.0),
            body_uv(outer),
            part_expand,
        ),
        box_spec(
            V3::new(-4.0 - arm_w, 12.0, -2.0),
            V3::new(-4.0, 24.0, 2.0),
            right_arm,
            part_expand,
        ),
        box_spec(
            V3::new(4.0, 12.0, -2.0),
            V3::new(4.0 + arm_w, 24.0, 2.0),
            left_arm,
            part_expand,
        ),
        box_spec(
            V3::new(-3.9, 0.0, -2.1),
            V3::new(0.1, 12.0, 1.9),
            right_leg,
            part_expand,
        ),
        box_spec(
            V3::new(0.1, 0.0, -2.1),
            V3::new(3.9, 12.0, 1.9),
            left_leg,
            part_expand,
        ),
    ]
}

/// Height of the skin texture in pixels, used to normalize UV coordinates.
fn skin_texture_height(legacy: bool) -> f32 {
    if legacy { 32.0 } else { 64.0 }
}

fn cape_boxes() -> Vec<BoxSpec> {
    // A 10x16x1 cape is pinned to the upper back at y=24, z=-2 and tilted 10.8 degrees so the
    // lower edge hangs away from the player's back. The yaw turns the outside face toward -z.
    let uvs = BoxUv {
        top: uv(1.0, 0.0, 11.0, 1.0),
        bottom: uv(11.0, 0.0, 21.0, 1.0),
        front: uv(1.0, 1.0, 11.0, 17.0),
        back: uv(12.0, 1.0, 22.0, 17.0),
        right: uv(11.0, 1.0, 12.0, 17.0),
        left: uv(0.0, 1.0, 1.0, 17.0),
    };
    vec![BoxSpec {
        min: V3::new(-5.0, 8.0, -2.0),
        max: V3::new(5.0, 24.0, -1.0),
        uv: uvs,
        transform: Transform {
            pivot: V3::new(0.0, 24.0, -2.0),
            yaw: std::f32::consts::PI,
            pitch: 10.8_f32.to_radians(),
            roll: 0.0,
        },
    }]
}

fn mirror_uv_x(region: Uv) -> Uv {
    uv(region.x1, region.y0, region.x0, region.y1)
}

fn mirror_uvs_x(uvs: BoxUv) -> BoxUv {
    BoxUv {
        top: mirror_uv_x(uvs.top),
        bottom: mirror_uv_x(uvs.bottom),
        front: mirror_uv_x(uvs.front),
        back: mirror_uv_x(uvs.back),
        right: mirror_uv_x(uvs.left),
        left: mirror_uv_x(uvs.right),
    }
}

fn elytra_uv() -> BoxUv {
    // Java Edition maps the 12x22x4 wing box with setCapeUVs(22, 0, 10, 20, 2): a 10x20 panel,
    // 2-pixel edge strips, and a mirrored 10x20 back panel on the 64x32 cape sheet. Importing a
    // real elytra PNG must keep this exact atlas; the UVs must never be nudged to match a test
    // texture.
    BoxUv {
        top: uv(24.0, 0.0, 34.0, 2.0),
        bottom: uv(34.0, 0.0, 44.0, 2.0),
        front: uv(24.0, 2.0, 34.0, 22.0),
        back: uv(36.0, 2.0, 46.0, 22.0),
        right: uv(34.0, 2.0, 36.0, 22.0),
        left: uv(22.0, 2.0, 24.0, 22.0),
    }
}

fn elytra_boxes() -> Vec<BoxSpec> {
    // Each wing is a 12x22x4 box hinged at the shoulders (x=+/-5, y=24, z=-2), tilted 15 degrees
    // forward and 15 degrees outward, matching the vanilla standing pose.
    let spec = |pivot_x: f32, yaw: f32, roll: f32, uvs: BoxUv| BoxSpec {
        min: V3::new(-6.0, 3.0, -5.0),
        max: V3::new(6.0, 25.0, -1.0),
        uv: uvs,
        transform: Transform {
            pivot: V3::new(pivot_x, 24.0, -2.0),
            yaw,
            pitch: 15.0_f32.to_radians(),
            roll,
        },
    };
    vec![
        spec(5.0, 0.01, 15.0_f32.to_radians(), elytra_uv()),
        spec(
            -5.0,
            -0.01,
            -15.0_f32.to_radians(),
            mirror_uvs_x(elytra_uv()),
        ),
    ]
}

fn solid_png() -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, 1, 1);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().expect("png header");
        writer
            .write_image_data(&[255, 255, 255, 255])
            .expect("png data");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::images::decode_png;
    use crate::test_support::{TempDir, encode_png};
    use std::fs;

    fn asymmetric_elytra_png() -> Vec<u8> {
        let mut rgba = [[45u8, 35, 60, 255]; 64 * 32];
        let mut fill = |x: usize, y: usize, w: usize, h: usize, c: [u8; 4]| {
            for yy in y..y + h {
                for xx in x..x + w {
                    rgba[yy * 64 + xx] = c;
                }
            }
        };
        fill(22, 2, 2, 20, [60, 110, 230, 255]);
        fill(24, 2, 5, 20, [230, 60, 60, 255]);
        fill(29, 2, 5, 20, [240, 210, 60, 255]);
        fill(34, 2, 2, 20, [60, 220, 220, 255]);
        fill(36, 2, 5, 20, [190, 80, 220, 255]);
        fill(41, 2, 5, 20, [60, 210, 90, 255]);
        fill(24, 0, 5, 2, [230, 60, 60, 255]);
        fill(29, 0, 5, 2, [240, 210, 60, 255]);
        fill(34, 0, 5, 2, [60, 210, 90, 255]);
        fill(39, 0, 5, 2, [60, 110, 230, 255]);
        let flat: Vec<u8> = rgba.into_iter().flatten().collect();
        encode_png(64, 32, &flat)
    }

    #[test]
    fn standard_head_uvs_match_the_skin_template() {
        let head = head_uv(false);
        assert_eq!(
            (head.top.x0, head.top.y0, head.top.x1, head.top.y1),
            (8.0, 0.0, 16.0, 8.0)
        );
        assert_eq!((head.front.x0, head.front.y0), (8.0, 8.0));
        assert_eq!((head.back.x0, head.back.y0), (24.0, 8.0));
        assert_eq!((head.right.x0, head.left.x0), (16.0, 0.0));
        let outer = head_uv(true);
        assert_eq!((outer.top.x0, outer.bottom.x0), (40.0, 48.0));
        assert_eq!((outer.front.x0, outer.front.y0), (40.0, 8.0));
        assert_eq!((outer.right.x0, outer.left.x0), (48.0, 32.0));
    }

    #[test]
    fn limb_and_torso_uvs_use_the_standard_layout() {
        let body = body_uv(false);
        assert_eq!((body.right.x0, body.left.x0), (28.0, 16.0));
        assert_eq!(body_uv(true).top.y0, 32.0);

        let right = right_arm_uv(SkinModel::Classic, false);
        assert_eq!((right.left.x0, right.right.x0), (40.0, 48.0));
        assert_eq!((right.back.x0, right.back.x1), (52.0, 56.0));
        let left = left_arm_uv(SkinModel::Classic, false);
        assert_eq!((left.top.x0, left.bottom.x0), (36.0, 40.0));
        assert_eq!((left.right.x0, left.left.x0), (40.0, 32.0));
        let left_overlay = left_arm_uv(SkinModel::Classic, true);
        assert_eq!((left_overlay.top.x0, left_overlay.back.x1), (52.0, 64.0));

        let right_leg = right_leg_uv(false);
        assert_eq!((right_leg.left.x0, right_leg.right.x0), (0.0, 8.0));
        let left_leg = left_leg_uv(false);
        assert_eq!((left_leg.top.x0, left_leg.front.x0), (20.0, 20.0));
        let left_leg_overlay = left_leg_uv(true);
        assert_eq!(
            (left_leg_overlay.top.x0, left_leg_overlay.left.x0),
            (4.0, 0.0)
        );
    }

    #[test]
    fn classic_and_slim_arms_have_the_right_widths() {
        let classic = right_arm_uv(SkinModel::Classic, false);
        let slim = right_arm_uv(SkinModel::Slim, false);
        assert_eq!(classic.front.x1 - classic.front.x0, 4.0);
        assert_eq!(slim.front.x1 - slim.front.x0, 3.0);
        assert_eq!((slim.top.x0, slim.top.x1), (44.0, 47.0));
        assert_eq!((slim.bottom.x0, slim.bottom.x1), (48.0, 51.0));
        assert_eq!((slim.back.x0, slim.back.x1), (52.0, 55.0));
        let slim_left = left_arm_uv(SkinModel::Slim, false);
        assert_eq!((slim_left.top.x0, slim_left.bottom.x0), (36.0, 40.0));
        assert_eq!((slim_left.front.x1, slim_left.back.x1), (39.0, 47.0));
        let parts = player_boxes(SkinModel::Slim, true, false);
        assert_eq!(parts.len(), 6);
        assert!(player_boxes(SkinModel::Classic, true, true).is_empty());
    }

    #[test]
    fn legacy_skins_mirror_the_right_limbs_and_use_32_pixel_uvs() {
        assert_eq!(player_boxes(SkinModel::Classic, true, true).len(), 0);
        let boxes = player_boxes(SkinModel::Classic, false, true);
        assert_eq!(boxes.len(), 6);
        let right_arm = right_arm_uv(SkinModel::Classic, false);
        let left_arm = boxes[3].uv;
        let flip = |u: Uv| uv(u.x1, u.y0, u.x0, u.y1);
        assert_eq!(left_arm.front, flip(right_arm.front));
        assert_eq!(left_arm.right, flip(right_arm.left));
        assert_eq!(left_arm.left, flip(right_arm.right));
        assert_eq!(left_arm.right.x0, 44.0);
        assert_eq!(left_arm.left.x0, 52.0);
        let right_leg = right_leg_uv(false);
        let left_leg = boxes[5].uv;
        assert_eq!(left_leg.front, flip(right_leg.front));
        assert_eq!(left_leg.right, flip(right_leg.left));
        assert_eq!(left_leg.left, flip(right_leg.right));
        assert!(
            boxes
                .iter()
                .all(|b| b.uv.front.y1 <= 32.0 && b.uv.top.y1 <= 32.0)
        );
        assert_eq!(skin_texture_height(true), 32.0);
        assert_eq!(skin_texture_height(false), 64.0);
    }

    #[test]
    fn legacy_skins_ignore_the_slim_model_setting() {
        let boxes = player_boxes(SkinModel::Slim, false, true);
        assert_eq!(boxes[2].max.x - boxes[2].min.x, 4.0);
        assert_eq!(boxes[2].uv.front.x1 - boxes[2].uv.front.x0, 4.0);
        assert_eq!(boxes[3].uv.front.x0 - boxes[3].uv.front.x1, 4.0);
    }

    #[test]
    fn player_parts_use_minecraft_32_pixel_coordinates() {
        let parts = player_boxes(SkinModel::Classic, false, false);
        assert_eq!((parts[0].min.y, parts[0].max.y), (24.0, 32.0));
        assert_eq!((parts[0].max.x - parts[0].min.x), 8.0);
        assert_eq!((parts[1].min.y, parts[1].max.y), (12.0, 24.0));
        assert_eq!((parts[2].min.y, parts[2].max.y), (12.0, 24.0));
        assert_eq!((parts[3].min.y, parts[3].max.y), (12.0, 24.0));
        assert_eq!((parts[4].min.y, parts[4].max.y), (0.0, 12.0));
        assert_eq!(parts[4].min, V3::new(-3.9, 0.0, -2.1));
        assert_eq!(parts[5].max, V3::new(3.9, 12.0, 1.9));
        assert_eq!(
            Projection::player(0.0, 0.0, pos2(0.0, 0.0), 1.0).center.y,
            16.0
        );
    }

    #[test]
    fn outer_layers_use_vanilla_thickness() {
        let outer = player_boxes(SkinModel::Classic, true, false);
        assert_eq!(outer[0].min, V3::new(-4.5, 23.5, -4.5));
        assert_eq!(outer[0].max, V3::new(4.5, 32.5, 4.5));
        assert_eq!(outer[1].min, V3::new(-4.25, 11.75, -2.25));
        assert_eq!(outer[1].max, V3::new(4.25, 24.25, 2.25));
        assert_eq!(outer[2].min, V3::new(-8.25, 11.75, -2.25));
        assert_eq!(outer[2].max, V3::new(-3.75, 24.25, 2.25));
        let slim = player_boxes(SkinModel::Slim, true, false);
        assert_eq!(slim[2].min.x, -7.25);
        assert_eq!(slim[3].max.x, 7.25);
        assert!(player_boxes(SkinModel::Classic, true, true).is_empty());
    }

    #[test]
    fn default_view_shows_the_top_of_the_model() {
        let state = State::default();
        assert!(state.pitch > 0.0);
        assert!(rotate(V3::new(0.0, 1.0, 0.0), 0.0, state.pitch).z > 0.0);
        assert!(rotate(V3::new(0.0, -1.0, 0.0), 0.0, state.pitch).z < 0.0);
    }

    #[test]
    fn cape_is_pinned_to_the_upper_back_and_flares_outward() {
        let cape = cape_boxes();
        assert_eq!(cape.len(), 1);
        let spec = cape[0];
        assert_eq!(
            (spec.max.x - spec.min.x, spec.max.y - spec.min.y),
            (10.0, 16.0)
        );
        assert_eq!(spec.transform.pivot, V3::new(0.0, 24.0, -2.0));
        assert_eq!(spec.min.y, 8.0);
        assert_eq!(spec.max.y, 24.0);
        let top = spec.transform.apply(V3::new(0.0, 24.0, -2.0));
        let bottom = spec.transform.apply(V3::new(0.0, 8.0, -1.0));
        assert_eq!(top, V3::new(0.0, 24.0, -2.0), "top stays at the shoulders");
        assert!(
            bottom.z < top.z - 2.0,
            "lower edge flares behind the player"
        );
    }

    #[test]
    fn elytra_wings_hinge_at_both_shoulders_and_mirror_their_uvs() {
        let wings = elytra_boxes();
        assert_eq!(wings.len(), 2);
        assert_eq!(wings[0].transform.pivot, V3::new(5.0, 24.0, -2.0));
        assert_eq!(wings[1].transform.pivot, V3::new(-5.0, 24.0, -2.0));
        assert_eq!(wings[0].transform.roll, -wings[1].transform.roll);
        assert_eq!(wings[0].transform.yaw, -wings[1].transform.yaw);
        assert_eq!(wings[0].transform.pitch, wings[1].transform.pitch);
        assert_eq!((wings[0].uv.top.x0, wings[0].uv.top.x1), (24.0, 34.0));
        assert_eq!((wings[0].uv.bottom.x0, wings[0].uv.bottom.x1), (34.0, 44.0));
        assert_eq!((wings[0].uv.front.x0, wings[0].uv.front.x1), (24.0, 34.0));
        assert_eq!((wings[0].uv.back.x0, wings[0].uv.back.x1), (36.0, 46.0));
        assert_eq!((wings[0].uv.right.x0, wings[0].uv.right.x1), (34.0, 36.0));
        assert_eq!((wings[0].uv.left.x0, wings[0].uv.left.x1), (22.0, 24.0));
        assert_eq!((wings[1].uv.front.x0, wings[1].uv.front.x1), (34.0, 24.0));
        assert_eq!((wings[1].uv.back.x0, wings[1].uv.back.x1), (46.0, 36.0));
        assert!(wings.iter().all(|w| w.uv.back.y1 <= 32.0));
        let hinge = wings[0].transform.apply(V3::new(5.0, 24.0, -2.0));
        assert_eq!(hinge, V3::new(5.0, 24.0, -2.0));
    }

    #[test]
    fn imported_elytra_png_uses_the_vanilla_atlas_and_mirrors_the_second_wing() {
        let dir = TempDir::new();
        let source = dir.path().join("uploaded-elytra.png");
        let destination = dir.path().join("custom").join("elytra.png");
        fs::write(&source, asymmetric_elytra_png()).unwrap();

        let info = skin::import_texture(&source, &destination, TextureKind::Elytra).unwrap();
        assert_eq!((info.width, info.height), (64, 32));
        let bytes = fs::read(&destination).unwrap();
        assert_eq!(
            bytes,
            fs::read(&source).unwrap(),
            "import must not resize or recolor pixels"
        );
        let decoded = decode_png(&bytes, 64).unwrap();
        assert_eq!((decoded.width, decoded.height), (64, 32));

        let sample = |region: Uv, fu: f32, fv: f32| -> [u8; 4] {
            let p = pos2(
                (region.x0 + (region.x1 - region.x0) * fu) / 64.0,
                (region.y0 + (region.y1 - region.y0) * fv) / 32.0,
            );
            let x = (p.x * decoded.width as f32) as usize;
            let y = (p.y * decoded.height as f32) as usize;
            let i = (y * decoded.width + x) * 4;
            [
                decoded.rgba[i],
                decoded.rgba[i + 1],
                decoded.rgba[i + 2],
                decoded.rgba[i + 3],
            ]
        };

        let wings = elytra_boxes();
        let first = wings[0].uv;
        assert_eq!(sample(first.front, 0.25, 0.5), [230, 60, 60, 255]);
        assert_eq!(sample(first.front, 0.75, 0.5), [240, 210, 60, 255]);
        assert_eq!(sample(first.back, 0.25, 0.5), [190, 80, 220, 255]);
        assert_eq!(sample(first.back, 0.75, 0.5), [60, 210, 90, 255]);
        assert_eq!(sample(first.left, 0.5, 0.5), [60, 110, 230, 255]);
        assert_eq!(sample(first.right, 0.5, 0.5), [60, 220, 220, 255]);
        assert_eq!(sample(first.top, 0.25, 0.5), [230, 60, 60, 255]);
        assert_eq!(sample(first.bottom, 0.25, 0.5), [60, 210, 90, 255]);

        let mirrored = wings[1].uv;
        assert_eq!(sample(mirrored.front, 0.25, 0.5), [240, 210, 60, 255]);
        assert_eq!(sample(mirrored.back, 0.25, 0.5), [60, 210, 90, 255]);
        assert_eq!(sample(mirrored.left, 0.5, 0.5), [60, 220, 220, 255]);
        assert_eq!(sample(mirrored.right, 0.5, 0.5), [60, 110, 230, 255]);
    }

    #[test]
    fn back_equipment_draw_order_follows_a_full_rotation() {
        assert!(!back_equipment_in_front(DEFAULT_YAW));
        assert!(!back_equipment_in_front(std::f32::consts::FRAC_PI_2));
        assert!(back_equipment_in_front(std::f32::consts::PI));
        assert!(!back_equipment_in_front(TAU - 0.01));
    }

    #[test]
    fn drop_zones_choose_the_right_texture() {
        let state = State {
            skin_drop_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(10.0, 10.0))),
            cape_drop_rect: Some(Rect::from_min_size(pos2(20.0, 0.0), vec2(10.0, 10.0))),
            elytra_drop_rect: Some(Rect::from_min_size(pos2(40.0, 0.0), vec2(10.0, 10.0))),
            ..Default::default()
        };
        assert_eq!(drop_kind(&state, pos2(5.0, 5.0)), Some(TextureKind::Skin));
        assert_eq!(drop_kind(&state, pos2(25.0, 5.0)), Some(TextureKind::Cape));
        assert_eq!(
            drop_kind(&state, pos2(45.0, 5.0)),
            Some(TextureKind::Elytra)
        );
        assert_eq!(drop_kind(&state, pos2(100.0, 5.0)), None);
    }
}
