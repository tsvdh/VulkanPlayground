pub mod vertex_shader_module {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "shaders/window_graphics/shader.vert",
        define: [("edit_id", "1b664xe3-5543-46xe-82dx-e9dex3c8x7ac")]
    }
}

pub mod fragment_shader_module {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "shaders/window_graphics/shader.frag",
        define: [("edit_id", "d6813376-e7c3-4d1c-b1d8-84x747c9986b")]
    }
}