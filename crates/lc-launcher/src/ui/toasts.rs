//! Transient notifications.

use egui::{Align2, Color32, Context, CornerRadius, Frame, Id, Margin, RichText, Stroke, Vec2};

use super::theme::Theme;
use super::widgets::ease_out;

/// Visual style of a toast.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastKind {
    /// Neutral information.
    Info,
    /// Something worked.
    Success,
    /// Something needs attention.
    Warning,
    /// Something failed.
    Error,
}

impl ToastKind {
    fn lifetime(self) -> f64 {
        match self {
            Self::Error | Self::Warning => 8.0,
            _ => 4.5,
        }
    }
}

#[derive(Debug, Clone)]
struct Toast {
    kind: ToastKind,
    text: String,
    born: f64,
    id: u64,
}

/// A queue of toasts.
#[derive(Debug, Default)]
pub struct Toasts {
    items: Vec<Toast>,
    clock: f64,
    next_id: u64,
}

const MAX_TOASTS: usize = 5;

impl Toasts {
    /// Update the clock used to timestamp new toasts.
    pub fn tick(&mut self, now: f64) {
        self.clock = now;
        self.items.retain(|t| now - t.born < t.kind.lifetime());
    }

    /// Show a toast.
    pub fn push(&mut self, kind: ToastKind, text: impl Into<String>) {
        let text = text.into();
        if self
            .items
            .iter()
            .any(|t| t.text == text && self.clock - t.born < 1.0)
        {
            return;
        }
        self.next_id += 1;
        self.items.push(Toast {
            kind,
            text,
            born: self.clock,
            id: self.next_id,
        });
        if self.items.len() > MAX_TOASTS {
            self.items.remove(0);
        }
    }

    /// Number of toasts currently queued.
    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Draw the toasts in the bottom-right corner; clicking one dismisses it.
    pub fn show(&mut self, ctx: &Context, theme: &Theme) {
        let mut dismiss = None;
        let mut offset = 0.0;
        for toast in self.items.iter().rev() {
            let age = self.clock - toast.born;
            let remaining = toast.kind.lifetime() - age;
            let alpha = ease_out((age / 0.25) as f32).min(ease_out((remaining / 0.4) as f32));
            let accent = match toast.kind {
                ToastKind::Info => theme.accent_hi,
                ToastKind::Success => theme.ok,
                ToastKind::Warning => theme.warn,
                ToastKind::Error => theme.danger,
            };
            let slide = (1.0 - ease_out((age / 0.25) as f32)) * 24.0;
            let area = egui::Area::new(Id::new(("toast", toast.id)))
                .anchor(
                    Align2::RIGHT_BOTTOM,
                    Vec2::new(-20.0 + slide, -20.0 - offset),
                )
                .order(egui::Order::Tooltip)
                .interactable(true)
                .show(ctx, |ui| {
                    ui.set_opacity(alpha);
                    Frame::new()
                        .fill(Color32::from_rgba_unmultiplied(24, 16, 40, 238))
                        .stroke(Stroke::new(1.0, accent.gamma_multiply(0.7)))
                        .corner_radius(CornerRadius::same(12))
                        .inner_margin(Margin::symmetric(16, 12))
                        .show(ui, |ui| {
                            ui.set_max_width(380.0);
                            ui.label(RichText::new(&toast.text).color(theme.text).size(14.5));
                        });
                });
            offset += area.response.rect.height() + 10.0;
            if area.response.clicked() {
                dismiss = Some(toast.id);
            }
        }
        if let Some(id) = dismiss {
            self.items.retain(|t| t.id != id);
        }
        if !self.items.is_empty() {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toasts_expire_and_are_capped() {
        let mut t = Toasts::default();
        t.tick(0.0);
        for i in 0..8 {
            t.push(ToastKind::Info, format!("message {i}"));
        }
        assert_eq!(t.len(), MAX_TOASTS);
        t.tick(100.0);
        assert_eq!(t.len(), 0);
    }

    #[test]
    fn identical_toasts_are_deduplicated_briefly() {
        let mut t = Toasts::default();
        t.tick(0.0);
        t.push(ToastKind::Error, "same");
        t.push(ToastKind::Error, "same");
        assert_eq!(t.len(), 1);
        t.tick(2.0);
        t.push(ToastKind::Error, "same");
        assert_eq!(t.len(), 2);
    }

    #[test]
    fn errors_live_longer_than_infos() {
        assert!(ToastKind::Error.lifetime() > ToastKind::Info.lifetime());
    }
}
