use std::time::Instant;
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

    fn process_input(&mut self) {
        if self.logic_items.keys_pressed.contains(&KeyCode::KeyT) {
            self.logic_items.show_frame_times = !self.logic_items.show_frame_times;
        }
    }

    fn get_frame_duration(&mut self) -> Option<f32> {
        if self.logic_items.previous_frame_logic_start.is_none() {
            self.logic_items.previous_frame_logic_start = Some(Instant::now());
            return None;
        }

        let frame_duration = self.logic_items.previous_frame_logic_start.unwrap().elapsed().as_secs_f32();
        self.logic_items.previous_frame_logic_start = Some(Instant::now());

        Some(frame_duration)
    }

    pub fn frame_logic(&mut self) {
        let frame_duration = match self.get_frame_duration() {
            Some(d) => d,
            None => return
        };

        self.process_input();



        self.logic_items.keys_pressed.clear();
    }
}