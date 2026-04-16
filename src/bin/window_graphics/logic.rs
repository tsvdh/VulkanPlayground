use std::f32::consts::FRAC_PI_2;
use std::time::Instant;
use glam::{Mat4, Vec3};
use log::info;
use winit::event::{KeyEvent};
use winit::keyboard::{KeyCode, PhysicalKey};
use crate::{App};
use crate::shader_modules::vertex_shader_module;

impl App {

    pub fn process_keyboard_input(&mut self, event: KeyEvent) {
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

    fn get_frame_duration(&mut self) -> Option<f32> {
        if self.logic_items.previous_frame_logic_start.is_none() {
            self.logic_items.previous_frame_logic_start = Some(Instant::now());
            return None;
        }

        let frame_duration = self.logic_items.previous_frame_logic_start.unwrap().elapsed().as_secs_f32();
        self.logic_items.previous_frame_logic_start = Some(Instant::now());

        Some(frame_duration)
    }

    fn make_mvp_matrix(&self) -> Mat4 {
        let image_extent = self.render_context.as_ref().unwrap().swapchain.image_extent();
        let aspect_ratio = image_extent[0] as f32 / image_extent[1] as f32;
        let projection = Mat4::perspective_lh(
            FRAC_PI_2,
            aspect_ratio,
            0.1,
            1000.0
        );

        let target = Vec3::new(0.0, 0.0, 0.0);
        let eye = Vec3::new(0.0, 0.0, -3.0);
        let view = Mat4::look_at_lh(
            eye,
            target,
            Vec3::Y
        );

        let model = Mat4::IDENTITY;

        // info!("{:?}", model);
        // info!("{:?}", view);
        // info!("{:?}", projection);
        // info!("{:?}", projection * (view * model));

        projection * (view * model)
    }

    pub fn frame_logic(&mut self) {
        self.logic_items.frame_id += 1;

        let frame_duration = self.get_frame_duration().unwrap_or(0.001);

        self.handle_input();



        let data = vertex_shader_module::Data {
            mvp: self.make_mvp_matrix().to_cols_array_2d()
        };
        self.logic_items.uniform_buffer = Some(self.uniform_buffer_allocator.allocate_sized().unwrap());
        *self.logic_items.uniform_buffer.as_mut().unwrap().write().unwrap() = data;

        self.logic_items.keys_pressed.clear();
    }
}