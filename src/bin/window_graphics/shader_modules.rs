pub mod vertex_shader_module {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "shaders/window_graphics/shader.vert",
        define: [("edit_id", "x3axd87x-xcxx-4axa-aaex-833993bdx87d")]
    }
}

pub mod fragment_shader_module {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "shaders/window_graphics/shader.frag",
        define: [("edit_id", "c9xeb1e7-93bc-4152-853b-cb5d6d4c315e")]
    }
}