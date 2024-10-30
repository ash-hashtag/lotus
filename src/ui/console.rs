use egui::{Color32, RichText};
use log::info;
use winit::event_loop::EventLoopProxy;

use crate::CustomEvents;

use super::renderer::UiNode;

pub const HISTORY_CAPACITY: usize = 2048;
pub const HISTORY_LENGTH: usize = 1024;

pub struct ConsoleNode {
    history: String,
    command: String,
    proxy: EventLoopProxy<CustomEvents>,
    request_focus: bool,
}

impl ConsoleNode {
    pub fn new(proxy: EventLoopProxy<CustomEvents>) -> Self {
        Self {
            command: String::new(),
            history: String::with_capacity(HISTORY_CAPACITY),
            proxy,
            request_focus: false,
        }
    }

    pub fn add_to_history(&mut self, s: &str) {
        if self.history.len() + s.len() > HISTORY_CAPACITY {
            let mut new_history = String::with_capacity(HISTORY_CAPACITY);
            if let Some(index) = self.history[self.history.len() - HISTORY_LENGTH..].find("\n") {
                new_history += &self.history[self.history.len() - HISTORY_LENGTH + index + 1..];
            }
            self.history = new_history;
        }

        self.history += s;
        self.history += "\n";
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
                let cmd = self.command.clone();
                self.add_to_history(&cmd);
                self.proxy
                    .send_event(CustomEvents::UserCommand(cmd))
                    .unwrap();
                self.command.clear();
                re.request_focus();
            }
            let text = RichText::new(&self.history).color(Color32::BLACK);
            let _re = ui.label(text);
        });
    }
}
