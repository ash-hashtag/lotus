use log::info;
use winit::event_loop::EventLoopProxy;

use crate::CustomEvents;

use super::renderer::UiNode;

pub struct ConsoleNode {
    command: String,
    proxy: EventLoopProxy<CustomEvents>,
    request_focus: bool,
}

impl ConsoleNode {
    pub fn new(proxy: EventLoopProxy<CustomEvents>) -> Self {
        Self {
            command: String::new(),
            proxy,
            request_focus: false,
        }
    }

    pub fn clear(&mut self) {
        self.command.clear();
    }

    pub fn should_request_focus(&mut self) {
        self.request_focus = true;
    }
}

impl UiNode for ConsoleNode {
    fn add_ui(&mut self, ui: &mut egui::Ui) {
        ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
            let re = ui.text_edit_singleline(&mut self.command);
            if self.request_focus {
                re.request_focus();
                self.request_focus = false;
            }
            let is_submitted = re.ctx.input(|x| x.key_down(egui::Key::Enter));

            if is_submitted && re.lost_focus() {
                info!("Executing Command {}", self.command);
                self.proxy
                    .send_event(CustomEvents::UserCommand(self.command.clone()))
                    .unwrap();
                self.command.clear();
                re.request_focus();
            }
        });
    }
}
