use winit::event::{KeyEvent};
use winit::keyboard::{KeyCode, PhysicalKey};
use crate::App;

impl App {

    pub fn handle_keyboard_input(&mut self, event: KeyEvent) {
        if event.repeat == true {
            return;
        }

        match event.physical_key {
            PhysicalKey::Code(key_code) => {
                if event.state.is_pressed() {
                    self.logic_items.keys_pressed.insert(key_code);
                    self.logic_items.keys_down.insert(key_code);
                } else {
                    self.logic_items.keys_down.remove(&key_code);
                }
            }
            PhysicalKey::Unidentified(_) => {}
        }
    }

    fn handle_input(&mut self) {
        if self.logic_items.keys_pressed.contains(&KeyCode::KeyT) {
            self.logic_items.show_frame_times = !self.logic_items.show_frame_times;
        }
    }

    pub fn frame_logic(&mut self) {
        self.handle_input();



        self.logic_items.keys_pressed.clear();
    }
}