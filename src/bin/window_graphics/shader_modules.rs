pub mod vertex_shader_module {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "shaders/window_graphics/shader.vert",
        define: [("edit_id", "4566x388-7xb3-4132-9d5x-bax8a838a57e")]
    }
}

pub mod fragment_shader_module {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "shaders/window_graphics/shader.frag",
        define: [("edit_id", "c9xeb1e7-93bc-4152-853b-cb5d6d4c315e")]
    }
}