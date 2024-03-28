use super::renderer::UiNode;

#[derive(Default)]
pub struct SettingsNode {
    pub show_fps: bool,
    pub show_wireframe: bool,
    pub full_screen: bool,
}

impl UiNode for SettingsNode {
    fn add_ui(&mut self, ui: &mut egui::Ui) {
        let settings_header = egui::RichText::new("Settings")
            .heading()
            .color(egui::Color32::WHITE);

        ui.label(settings_header);
        if !self.show_fps {
            if ui.button("Show FPS").clicked() {
                self.show_fps = true;
            }
        } else {
            if ui.button("Hide FPS").clicked() {
                self.show_fps = false;
            }
        }

        if !self.show_wireframe {
            if ui.button("Show Wireframe").clicked() {
                self.show_wireframe = true;
            }
        } else {
            if ui.button("Hide Wireframe").clicked() {
                self.show_wireframe = false;
            }
        }

        if !self.full_screen {
            if ui.button("Toggle Full Screen").clicked() {
                self.full_screen = true;
            }
        } else {
            if ui.button("Untoggle Full Screen").clicked() {
                self.full_screen = false;
            }
        }
    }
}
