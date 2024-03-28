use std::time::Duration;

use super::renderer::UiNode;

pub struct DebugOverlay {
    pub dt: Duration,
}

impl UiNode for DebugOverlay {
    fn add_ui(&self, ui: &mut egui::Ui) {
        let fps = (1.0 / self.dt.as_secs_f32()).floor();
        let fps_text = format!("FPS: {fps}");
        let text = egui::RichText::new(fps_text)
            .color(egui::Color32::WHITE)
            .size(16.0);
        ui.label(text);
    }
}
