use std::sync::Arc;
use log::{info, warn};
use vulkano::buffer::{Buffer, BufferContents, BufferCreateInfo, BufferUsage, Subbuffer};
use vulkano::device::{DeviceExtensions, DeviceFeatures, QueueFlags};
use vulkano::image::{Image, ImageUsage};
use vulkano::image::view::ImageView;
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryTypeFilter};
use vulkano::pipeline::graphics::vertex_input::{Vertex, VertexDefinition};
use vulkano::pipeline::graphics::viewport::{Viewport, ViewportState};
use vulkano::pipeline::{DynamicState, GraphicsPipeline, PipelineLayout, PipelineShaderStageCreateInfo};
use vulkano::pipeline::graphics::color_blend::{ColorBlendAttachmentState, ColorBlendState};
use vulkano::pipeline::graphics::GraphicsPipelineCreateInfo;
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::multisample::MultisampleState;
use vulkano::pipeline::graphics::rasterization::RasterizationState;
use vulkano::pipeline::layout::{PipelineDescriptorSetLayoutCreateInfo};
use vulkano::render_pass::{AttachmentLoadOp, AttachmentStoreOp, Framebuffer, FramebufferCreateInfo, RenderPass, Subpass};
use vulkano::swapchain::{acquire_next_image, Surface, Swapchain, SwapchainCreateInfo, SwapchainPresentInfo};
use vulkano::{sync, Validated, VulkanError};
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage, RenderPassBeginInfo, RenderingAttachmentInfo, RenderingInfo, SubpassBeginInfo, SubpassContents, SubpassEndInfo};
use vulkano::pipeline::graphics::subpass::{PipelineRenderingCreateInfo, PipelineSubpassType};
use vulkano::sync::GpuFuture;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};
use VulkanPlayground::CommonItems;

fn main() {
    let event_loop = EventLoop::new().unwrap();
    let mut app = App::new(&event_loop);
    event_loop.run_app(&mut app).unwrap();
}

struct App {
    common_items: CommonItems,
    vertex_buffer: Subbuffer<[BasicVertex]>,
    render_context: Option<RenderContext>,
}

struct RenderContext {
    window: Arc<Window>,
    swapchain: Arc<Swapchain>,
    attachment_image_views: Vec<Arc<ImageView>>,
    pipeline: Arc<GraphicsPipeline>,
    viewport: Viewport,
    recreate_swapchain: bool,
    previous_frame_end: Option<Box<dyn GpuFuture>>,
}

#[derive(BufferContents, Vertex)]
#[repr(C)]
struct BasicVertex {
    #[format(R32G32_SFLOAT)]
    position: [f32; 2]
}

impl App {
    fn new(event_loop: &EventLoop<()>) -> Self {
        let instance_extensions = Surface::required_extensions(event_loop).unwrap();
        let device_extensions = DeviceExtensions {
            khr_swapchain: true,
            khr_dynamic_rendering: true,
            ..DeviceExtensions::empty()
        };
        let device_features = DeviceFeatures {
            dynamic_rendering: true,
            ..DeviceFeatures::empty()
        };

        let common_items = VulkanPlayground::get_common_vulkan_items(
            Some(instance_extensions),
            Some(device_extensions),
            Some(device_features),
            QueueFlags::GRAPHICS,
            Some(event_loop)
        );

        let vertex1 = BasicVertex { position: [0.0, -0.5]};
        let vertex2 = BasicVertex { position: [0.5, 0.0]};
        let vertex3 = BasicVertex { position: [-0.5, 0.0]};

        let vertex_buffer = Buffer::from_iter(
            common_items.memory_allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::VERTEX_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            vec![vertex1, vertex2, vertex3]
        ).unwrap();

        App {
            common_items,
            vertex_buffer,
            render_context: None
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(event_loop.create_window(Window::default_attributes()).unwrap());
        let surface = Surface::from_window(self.common_items.instance.clone(), window.clone()).unwrap();

        let (swapchain, images) = {
            let surface_capabilities = self.common_items.device.physical_device()
                .surface_capabilities(&surface, Default::default()).unwrap();

            let (image_format, _) = self.common_items.device.physical_device()
                .surface_formats(&surface, Default::default()).unwrap()[0];

            Swapchain::new(
                self.common_items.device.clone(),
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
            let vertex_shader_module = vertex_shader_module::load(self.common_items.device.clone()).expect("Failed to create vertex shader");
            let fragment_shader_module = fragment_shader_module::load(self.common_items.device.clone()).expect("Failed to create fragment shader");
            let vertex_shader = vertex_shader_module.entry_point("main").unwrap();
            let fragment_shader = fragment_shader_module.entry_point("main").unwrap();

            let vertex_input_state = BasicVertex::per_vertex().definition(&vertex_shader).unwrap();

            let stages = [
                PipelineShaderStageCreateInfo::new(vertex_shader),
                PipelineShaderStageCreateInfo::new(fragment_shader)
            ];

            let layout = PipelineLayout::new(
                self.common_items.device.clone(),
                PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages)
                    .into_pipeline_layout_create_info(self.common_items.device.clone()).unwrap()
            ).unwrap();

            let dynamic_rendering_info = PipelineRenderingCreateInfo {
                color_attachment_formats: vec![Some(swapchain.image_format())],
                ..Default::default()
            };

            GraphicsPipeline::new(
                self.common_items.device.clone(),
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

        let previous_frame_end = Some(sync::now(self.common_items.device.clone()).boxed());

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

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _window_id: WindowId, event: WindowEvent) {
        // self.render_context is guaranteed to be Some
        let render_context = self.render_context.as_mut().unwrap();

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(_) => {
                render_context.recreate_swapchain = true;
            }
            WindowEvent::RedrawRequested => {
                let new_window_size = render_context.window.inner_size();

                if new_window_size.width == 0 {
                    return;
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

                let (image_index, suboptimal, acquire_future) =
                    match acquire_next_image(render_context.swapchain.clone(), None).map_err(Validated::unwrap) {
                        Ok(result) => result,
                        Err(VulkanError::OutOfDate) => {
                            render_context.recreate_swapchain = true;
                            return;
                        },
                        Err(error) => panic!("Failed to acquire next image: {error}")
                    };

                if suboptimal {
                    render_context.recreate_swapchain = true;
                    return;
                }

                info!("Executing render pass");

                let mut command_buffer_builder = AutoCommandBufferBuilder::primary(
                    self.common_items.command_buffer_allocator.clone(),
                    self.common_items.queue.queue_family_index(),
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
                    .then_execute(self.common_items.queue.clone(), command_buffer.clone()).unwrap()
                    .then_swapchain_present(self.common_items.queue.clone(),
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
                        render_context.previous_frame_end = Some(sync::now(self.common_items.device.clone()).boxed());
                        warn!("Rendering failed: {error}");
                    }
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        info!("About to wait event, requesting redraw");
        self.render_context.as_mut().unwrap().window.request_redraw();
    }
}

fn make_image_views(images: &[Arc<Image>]) -> Vec<Arc<ImageView>> {
    images.iter().map(|image| {
        ImageView::new_default(image.clone()).unwrap()
    }).collect()
}