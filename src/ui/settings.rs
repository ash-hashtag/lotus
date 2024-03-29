use super::renderer::UiNode;

#[derive(Default)]
pub struct SettingsNode {
    pub show_fps: bool,
    pub show_wireframe: bool,
    pub full_screen: bool,
    pub show_noise: bool,
}

impl UiNode for SettingsNode {
    fn add_ui(&mut self, ui: &mut egui::Ui) {
        let settings_header = egui::RichText::new("Settings")
            .heading()
            .color(egui::Color32::WHITE);

        ui.label(settings_header);
        ui.toggle_value(&mut self.show_fps, "Show FPS");
        ui.toggle_value(&mut self.show_wireframe, "Show Wireframe");
        ui.toggle_value(&mut self.show_noise, "Show Noise");
        ui.toggle_value(&mut self.full_screen, "FullScreen");
    }
}
