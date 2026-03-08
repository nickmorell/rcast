use std::time::{Duration, Instant};

use egui::Context;

#[derive(Debug, Clone)]
pub enum ToastKind {
    Success,
    Error,
    Info,
}

#[derive(Clone)]
pub struct ToastMessage {
    pub kind: ToastKind,
    pub text: String,
    pub created_at: Instant,
    pub duration: Duration,
}

impl std::fmt::Debug for ToastMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Toast({:?}: {})", self.kind, self.text)
    }
}

impl ToastMessage {
    pub fn success(text: &str) -> Self {
        Self::new(ToastKind::Success, text, Duration::from_secs(3))
    }

    pub fn error(text: &str) -> Self {
        Self::new(ToastKind::Error, text, Duration::from_secs(5))
    }

    pub fn info(text: &str) -> Self {
        Self::new(ToastKind::Info, text, Duration::from_secs(3))
    }

    fn new(kind: ToastKind, text: &str, duration: Duration) -> Self {
        Self {
            kind,
            text: text.to_string(),
            created_at: Instant::now(),
            duration,
        }
    }

    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() >= self.duration
    }
}

#[derive(Default)]
pub struct ToastQueue {
    toasts: Vec<ToastMessage>,
}

impl ToastQueue {
    pub fn push(&mut self, msg: ToastMessage) {
        self.toasts.push(msg);
    }
}

// Renders active toasts in the bottom-right corner and removes expired ones.
pub fn render(ctx: &Context, queue: &mut ToastQueue) {
    queue.toasts.retain(|t| !t.is_expired());

    if queue.toasts.is_empty() {
        return;
    }

    // Keep repainting so toasts disappear on time even when idle.
    ctx.request_repaint();

    let screen = ctx.content_rect();
    let mut y_offset = screen.max.y - 12.0;

    for toast in queue.toasts.iter().rev() {
        let bg_color = match toast.kind {
            ToastKind::Success => egui::Color32::from_rgb(34, 139, 34),
            ToastKind::Error => egui::Color32::from_rgb(180, 40, 40),
            ToastKind::Info => egui::Color32::from_rgb(50, 100, 180),
        };

        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Middle,
            egui::Id::new("toast_text"),
        ));
        let galley = painter.layout_no_wrap(
            toast.text.clone(),
            egui::FontId::proportional(14.0),
            egui::Color32::WHITE,
        );

        let padding = egui::vec2(12.0, 8.0);
        let size = galley.size() + padding * 2.0;
        let rect = egui::Rect::from_min_size(
            egui::pos2(screen.max.x - size.x - 12.0, y_offset - size.y),
            size,
        );

        egui::Area::new(format!("toast_{}", toast.text).into())
            .fixed_pos(rect.min)
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(bg_color)
                    .corner_radius(egui::CornerRadius::same(6))
                    .inner_margin(egui::Margin::symmetric(12, 8))
                    .show(ui, |ui| {
                        ui.colored_label(egui::Color32::WHITE, &toast.text);
                    });
            });

        y_offset -= size.y + 6.0;
    }
}
