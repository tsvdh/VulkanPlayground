pub mod vertex_shader_module {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "shaders/window_graphics/shader.vert",
        define: [("edit_id", "9x9cxx21-xd11-492e-8b2d-e54a1exex89d")]
    }
}

pub mod fragment_shader_module {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "shaders/window_graphics/shader.frag",
        define: [("edit_id", "d6813376-e7c3-4d1c-b1d8-84x747c9986b")]
    }
}