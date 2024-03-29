use log::info;

use super::renderer::UiNode;

pub struct ConsoleNode {
    command: String,
}

impl UiNode for ConsoleNode {
    fn add_ui(&mut self, ui: &mut egui::Ui) {
        if ui.text_edit_singleline(&mut self.command).lost_focus() {
            info!("Executing Command {}", self.command);
            self.command.clear();
        }
    }
}
