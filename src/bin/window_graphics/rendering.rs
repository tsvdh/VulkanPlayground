use std::sync::Arc;
use log::{info, warn};
use vulkano::image::ImageUsage;
use vulkano::pipeline::{DynamicState, GraphicsPipeline, PipelineLayout, PipelineShaderStageCreateInfo};
use vulkano::pipeline::graphics::color_blend::{ColorBlendAttachmentState, ColorBlendState};
use vulkano::pipeline::graphics::GraphicsPipelineCreateInfo;
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::multisample::MultisampleState;
use vulkano::pipeline::graphics::rasterization::RasterizationState;
use vulkano::pipeline::graphics::subpass::PipelineRenderingCreateInfo;
use vulkano::pipeline::graphics::vertex_input::{Vertex, VertexDefinition};
use vulkano::pipeline::graphics::viewport::{Viewport, ViewportState};
use vulkano::pipeline::layout::PipelineDescriptorSetLayoutCreateInfo;
use vulkano::swapchain::{acquire_next_image, Surface, Swapchain, SwapchainAcquireFuture, SwapchainCreateInfo, SwapchainPresentInfo};
use vulkano::{sync, Validated, VulkanError};
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage, RenderingAttachmentInfo, RenderingInfo};
use vulkano::render_pass::{AttachmentLoadOp, AttachmentStoreOp};
use vulkano::sync::GpuFuture;
use winit::event_loop::ActiveEventLoop;
use winit::window::Window;
use crate::{make_image_views, App, BasicVertex, RenderContext};

impl App {
    pub fn init_render_context(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(event_loop.create_window(Window::default_attributes()).unwrap());
        window.set_title("VulkanPlayground");

        let surface = Surface::from_window(self.vulkan_items.instance.clone(), window.clone()).unwrap();

        let (swapchain, images) = {
            let surface_capabilities = self.vulkan_items.device.physical_device()
                .surface_capabilities(&surface, Default::default()).unwrap();

            let (image_format, _) = self.vulkan_items.device.physical_device()
                .surface_formats(&surface, Default::default()).unwrap()[0];

            Swapchain::new(
                self.vulkan_items.device.clone(),
                surface.clone(),
                SwapchainCreateInfo {
                    min_image_count: surface_capabilities.min_image_count.max(2),
                    image_format,
                    image_extent: window.inner_size().into(),
                    image_usage: ImageUsage::COLOR_ATTACHMENT,
                    ..Default::default()
                }
            ).unwrap()
        };

        let attachment_image_views = make_image_views(&images);

        let pipeline = {
            mod vertex_shader_module {
                vulkano_shaders::shader! {
                    ty: "vertex",
                    path: "shaders/window_graphics/shader.vert"
                }
            }
            mod fragment_shader_module {
                vulkano_shaders::shader! {
                    ty: "fragment",
                    path: "shaders/window_graphics/shader.frag"
                }
            }
            let vertex_shader_module = vertex_shader_module::load(self.vulkan_items.device.clone()).expect("Failed to create vertex shader");
            let fragment_shader_module = fragment_shader_module::load(self.vulkan_items.device.clone()).expect("Failed to create fragment shader");
            let vertex_shader = vertex_shader_module.entry_point("main").unwrap();
            let fragment_shader = fragment_shader_module.entry_point("main").unwrap();

            let vertex_input_state = BasicVertex::per_vertex().definition(&vertex_shader).unwrap();

            let stages = [
                PipelineShaderStageCreateInfo::new(vertex_shader),
                PipelineShaderStageCreateInfo::new(fragment_shader)
            ];

            let layout = PipelineLayout::new(
                self.vulkan_items.device.clone(),
                PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages)
                    .into_pipeline_layout_create_info(self.vulkan_items.device.clone()).unwrap()
            ).unwrap();

            let dynamic_rendering_info = PipelineRenderingCreateInfo {
                color_attachment_formats: vec![Some(swapchain.image_format())],
                ..Default::default()
            };

            GraphicsPipeline::new(
                self.vulkan_items.device.clone(),
                None,
                GraphicsPipelineCreateInfo {
                    stages: stages.into_iter().collect(),
                    vertex_input_state: Some(vertex_input_state),
                    input_assembly_state: Some(InputAssemblyState::default()),
                    viewport_state: Some(ViewportState::default()),
                    rasterization_state: Some(RasterizationState::default()),
                    multisample_state: Some(MultisampleState::default()),
                    color_blend_state: Some(ColorBlendState::with_attachment_states(
                        dynamic_rendering_info.color_attachment_formats.len() as u32,
                        ColorBlendAttachmentState::default()
                    )),
                    dynamic_state: [DynamicState::Viewport].into_iter().collect(),
                    subpass: Some(dynamic_rendering_info.into()),
                    ..GraphicsPipelineCreateInfo::layout(layout.clone())
                }
            ).unwrap()
        };

        let viewport = Viewport {
            offset: [0.0, 0.0],
            extent: window.inner_size().into(),
            depth_range: 0.0..=1.0
        };

        let previous_frame_end = Some(sync::now(self.vulkan_items.device.clone()).boxed());

        self.render_context = Some(RenderContext {
            window,
            swapchain,
            attachment_image_views,
            pipeline,
            viewport,
            recreate_swapchain: false,
            previous_frame_end,
        });
    }

    pub fn frame_prep(&mut self) -> Option<SwapchainAcquireFuture> {
        let render_context = self.render_context.as_mut().unwrap();

        let new_window_size = render_context.window.inner_size();
        if new_window_size.width == 0 {
            return None;
        }
        render_context.previous_frame_end.as_mut().unwrap().cleanup_finished();

        if render_context.recreate_swapchain {
            info!("Recreating swapchain");
            let (new_swapchain, new_images) = render_context.swapchain.recreate(
                SwapchainCreateInfo {
                    image_extent: new_window_size.into(),
                    ..render_context.swapchain.create_info()
                }
            ).unwrap();

            render_context.swapchain = new_swapchain;
            render_context.attachment_image_views = make_image_views(&new_images);
            render_context.viewport.extent = new_window_size.into();
            render_context.recreate_swapchain = false;
        }

        let (_image_index, suboptimal, acquire_future) =
            match acquire_next_image(render_context.swapchain.clone(), None).map_err(Validated::unwrap) {
                Ok(result) => result,
                Err(VulkanError::OutOfDate) => {
                    render_context.recreate_swapchain = true;
                    return None;
                },
                Err(error) => panic!("Failed to acquire next image: {error}")
            };

        if suboptimal {
            render_context.recreate_swapchain = true;
            return None;
        }

        Some(acquire_future)
    }

    pub fn frame_render(&mut self, acquire_future: SwapchainAcquireFuture) {
        let render_context = self.render_context.as_mut().unwrap();
        let image_index = acquire_future.image_index();

        let mut command_buffer_builder = AutoCommandBufferBuilder::primary(
            self.vulkan_items.command_buffer_allocator.clone(),
            self.vulkan_items.queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit
        ).unwrap();

        command_buffer_builder
            .begin_rendering(
                RenderingInfo {
                    color_attachments: vec![Some(RenderingAttachmentInfo {
                        load_op: AttachmentLoadOp::Clear,
                        store_op: AttachmentStoreOp::Store,
                        clear_value: Some([0.0, 0.0, 0.0, 1.0].into()),
                        ..RenderingAttachmentInfo::image_view(render_context.attachment_image_views[image_index as usize].clone())
                    })],
                    ..Default::default()
                }
            ).unwrap()
            .set_viewport(0, [render_context.viewport.clone()].into_iter().collect()).unwrap()
            .bind_pipeline_graphics(render_context.pipeline.clone()).unwrap()
            .bind_vertex_buffers(0, self.vertex_buffer.clone()).unwrap();

        unsafe {
            command_buffer_builder.draw(3, 1, 0, 0).unwrap();
        }

        command_buffer_builder
            .end_rendering().unwrap();

        let command_buffer = command_buffer_builder.build().unwrap();

        let future = render_context.previous_frame_end.take().unwrap()
            .join(acquire_future)
            .then_execute(self.vulkan_items.queue.clone(), command_buffer.clone()).unwrap()
            .then_swapchain_present(self.vulkan_items.queue.clone(),
                                    SwapchainPresentInfo::swapchain_image_index(render_context.swapchain.clone(), image_index))
            .then_signal_fence_and_flush();

        match future.map_err(Validated::unwrap) {
            Ok(future) => {
                render_context.previous_frame_end = Some(future.boxed());
            }
            Err(error) => {
                if error == VulkanError::OutOfDate {
                    render_context.recreate_swapchain = true;
                }
                render_context.previous_frame_end = Some(sync::now(self.vulkan_items.device.clone()).boxed());
                warn!("Rendering failed: {error}");
            }
        }
    }
}