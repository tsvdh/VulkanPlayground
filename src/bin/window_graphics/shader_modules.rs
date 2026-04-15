pub mod vertex_shader_module {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "shaders/window_graphics/shader.vert",
        define: [("edit_id", "954919x5-133x-4665-9cdx-d251a78a3ae8")]
    }
}

pub mod fragment_shader_module {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "shaders/window_graphics/shader.frag",
        define: [("edit_id", "d6813376-e7c3-4d1c-b1d8-84x747c9986b")]
    }
}