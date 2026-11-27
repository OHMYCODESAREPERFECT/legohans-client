//! Black and purple theme.

use egui::{Color32, Context, CornerRadius, FontFamily, FontId, Stroke, TextStyle, Vec2};

use crate::model::settings::Accent;

/// Colors and animation switch used by every widget.
#[derive(Debug, Clone)]
pub struct Theme {
    /// Main accent.
    pub accent: Color32,
    /// Lighter accent for highlights.
    pub accent_hi: Color32,
    /// Page background.
    pub bg: Color32,
    /// Slightly lighter background for sidebars.
    pub bg_alt: Color32,
    /// Translucent glass fill.
    pub glass: Color32,
    /// Glass border.
    pub border: Color32,
    /// Primary text.
    pub text: Color32,
    /// Secondary text.
    pub text_dim: Color32,
    /// Hint text.
    pub text_faint: Color32,
    /// Positive state.
    pub ok: Color32,
    /// Warning state.
    pub warn: Color32,
    /// Error state.
    pub danger: Color32,
    /// Whether animations run.
    pub animations: bool,
}

fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    a.lerp_to_gamma(b, t)
}

impl Theme {
    /// Build the theme for an accent preset.
    pub fn new(accent: Accent, animations: bool) -> Self {
        let [r, g, b] = accent.color();
        let accent_color = Color32::from_rgb(r, g, b);
        Self {
            accent: accent_color,
            accent_hi: mix(accent_color, Color32::WHITE, 0.35),
            bg: Color32::from_rgb(8, 6, 14),
            bg_alt: Color32::from_rgb(12, 9, 20),
            glass: Color32::from_rgba_unmultiplied(26, 18, 44, 190),
            border: Color32::from_rgba_unmultiplied(176, 140, 255, 48),
            text: Color32::from_rgb(238, 232, 252),
            text_dim: Color32::from_rgb(172, 162, 198),
            text_faint: Color32::from_rgb(120, 110, 148),
            ok: Color32::from_rgb(88, 214, 141),
            warn: Color32::from_rgb(250, 190, 70),
            danger: Color32::from_rgb(246, 106, 118),
            animations,
        }
    }

    /// Animation duration, or zero when animations are off.
    pub fn anim(&self, secs: f32) -> f32 {
        if self.animations { secs } else { 0.0 }
    }

    /// The accent with a given alpha.
    pub fn accent_alpha(&self, alpha: u8) -> Color32 {
        let [r, g, b, _] = self.accent.to_array();
        Color32::from_rgba_unmultiplied(r, g, b, alpha)
    }

    /// Install visuals and text styles on the egui context.
    pub fn apply(&self, ctx: &Context) {
        // The launcher owns zoom: keyboard shortcuts must not fight the numeric setting.
        ctx.options_mut(|o| o.zoom_with_keyboard = false);
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = self.bg;
        visuals.window_fill = Color32::from_rgb(20, 14, 34);
        visuals.extreme_bg_color = Color32::from_rgb(10, 7, 18);
        visuals.faint_bg_color = Color32::from_rgba_unmultiplied(120, 90, 200, 14);
        visuals.window_stroke = Stroke::new(1.0, self.border);
        visuals.window_corner_radius = CornerRadius::same(18);
        visuals.menu_corner_radius = CornerRadius::same(12);
        visuals.hyperlink_color = self.accent_hi;
        visuals.selection.bg_fill = self.accent_alpha(110);
        visuals.selection.stroke = Stroke::new(1.0, self.accent_hi);

        let radius = CornerRadius::same(10);
        let idle = Color32::from_rgba_unmultiplied(60, 44, 96, 120);
        let w = &mut visuals.widgets;
        w.noninteractive.bg_stroke = Stroke::new(1.0, self.border);
        w.noninteractive.fg_stroke = Stroke::new(1.0, self.text_dim);
        w.inactive.bg_fill = idle;
        w.inactive.weak_bg_fill = idle;
        w.inactive.bg_stroke = Stroke::new(1.0, self.border);
        w.inactive.fg_stroke = Stroke::new(1.0, self.text);
        w.hovered.bg_fill = self.accent_alpha(70);
        w.hovered.weak_bg_fill = self.accent_alpha(70);
        w.hovered.bg_stroke = Stroke::new(1.0, self.accent);
        w.hovered.fg_stroke = Stroke::new(1.5, Color32::WHITE);
        w.active.bg_fill = self.accent_alpha(130);
        w.active.weak_bg_fill = self.accent_alpha(130);
        w.active.bg_stroke = Stroke::new(1.0, self.accent_hi);
        w.active.fg_stroke = Stroke::new(1.5, Color32::WHITE);
        w.open.bg_fill = self.accent_alpha(90);
        w.open.weak_bg_fill = self.accent_alpha(90);
        for state in [
            &mut w.noninteractive,
            &mut w.inactive,
            &mut w.hovered,
            &mut w.active,
            &mut w.open,
        ] {
            state.corner_radius = radius;
        }
        ctx.set_visuals(visuals);

        let mut style = (*ctx.style()).clone();
        style.spacing.item_spacing = Vec2::new(10.0, 10.0);
        style.spacing.button_padding = Vec2::new(14.0, 8.0);
        style.spacing.interact_size.y = 32.0;
        style.spacing.scroll.bar_width = 8.0;
        style.text_styles = [
            (
                TextStyle::Heading,
                FontId::new(30.0, FontFamily::Proportional),
            ),
            (TextStyle::Body, FontId::new(15.0, FontFamily::Proportional)),
            (
                TextStyle::Button,
                FontId::new(15.0, FontFamily::Proportional),
            ),
            (
                TextStyle::Small,
                FontId::new(12.5, FontFamily::Proportional),
            ),
            (
                TextStyle::Monospace,
                FontId::new(13.5, FontFamily::Monospace),
            ),
        ]
        .into();
        ctx.set_style(style);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accents_differ_and_animation_switch_works() {
        let a = Theme::new(Accent::Violet, true);
        let b = Theme::new(Accent::Indigo, false);
        assert_ne!(a.accent, b.accent);
        assert_eq!(a.anim(0.2), 0.2);
        assert_eq!(b.anim(0.2), 0.0);
    }

    #[test]
    fn custom_accent_uses_the_chosen_color() {
        let theme = Theme::new(Accent::custom([10, 20, 30]), true);
        assert_eq!(theme.accent, Color32::from_rgb(10, 20, 30));
    }

    #[test]
    fn apply_does_not_panic_and_disables_keyboard_zoom() {
        let ctx = Context::default();
        Theme::new(Accent::Orchid, true).apply(&ctx);
        assert!(!ctx.options(|o| o.zoom_with_keyboard));
    }
}
