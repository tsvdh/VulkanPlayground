use std::sync::Arc;
use vulkano::buffer::{Buffer, BufferContents, BufferCreateInfo, BufferUsage};
use vulkano::command_buffer::allocator::StandardCommandBufferAllocator;
use vulkano::device::{Device, DeviceExtensions, Queue, QueueFlags};
use vulkano::device::physical::PhysicalDevice;
use vulkano::image::{Image, ImageUsage};
use vulkano::image::view::ImageView;
use vulkano::instance::debug::DebugUtilsMessenger;
use vulkano::instance::Instance;
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryTypeFilter};
use vulkano::pipeline::graphics::vertex_input::{Vertex, VertexDefinition};
use vulkano::pipeline::graphics::viewport::{Viewport, ViewportState};
use vulkano::pipeline::{GraphicsPipeline, PipelineLayout, PipelineShaderStageCreateInfo};
use vulkano::pipeline::graphics::color_blend::{ColorBlendAttachmentState, ColorBlendState};
use vulkano::pipeline::graphics::GraphicsPipelineCreateInfo;
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::multisample::MultisampleState;
use vulkano::pipeline::graphics::rasterization::RasterizationState;
use vulkano::pipeline::layout::{PipelineDescriptorSetLayoutCreateInfo, PipelineLayoutCreateInfo};
use vulkano::render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass, Subpass};
use vulkano::shader::ShaderModule;
use vulkano::swapchain::{Surface, Swapchain, SwapchainCreateInfo};
use vulkano::VulkanLibrary;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

struct App {
    library: Arc<VulkanLibrary>,
    instance: Arc<Instance>,
    debug_callback: DebugUtilsMessenger,
    queue_family_index: u32,
    device: Arc<Device>,
    queue: Arc<Queue>,
    render_context: Option<RenderContext>
}

struct RenderContext {
    window: Arc<Window>
}

fn get_swapchain(device: Arc<Device>, surface: Arc<Surface>, window: Arc<Window>) -> (Arc<Swapchain>, Vec<Arc<Image>>) {
    let surface_capabilities = device.physical_device()
        .surface_capabilities(&surface, Default::default()).unwrap();

    let dimensions = window.inner_size();
    let composite_alpha = surface_capabilities.supported_composite_alpha.into_iter().next().unwrap();
    let image_format = device.physical_device().
        surface_formats(&surface, Default::default())
        .unwrap()[0].0;

    Swapchain::new(
        device.clone(),
        surface.clone(),
        SwapchainCreateInfo {
            min_image_count: surface_capabilities.min_image_count + 1,
            image_format,
            image_extent: dimensions.into(),
            image_usage: ImageUsage::COLOR_ATTACHMENT,
            composite_alpha,
            ..Default::default()
        }
    ).unwrap()
}

fn get_render_pass(device: Arc<Device>, swapchain: Arc<Swapchain>) -> Arc<RenderPass> {
    vulkano::single_pass_renderpass!(
        device.clone(),
        attachments: {
            color: {
                format: swapchain.image_format(),
                samples: 1,
                load_op: Clear,
                store_op: Store
            }
        },
        pass: {
            color: [color],
            depth_stencil: {}
        }
    ).unwrap()
}

fn get_framebuffers(images: Vec<Arc<Image>>, render_pass: Arc<RenderPass>) -> Vec<Arc<Framebuffer>> {
    images.iter()
        .map(|image| {
            let view = ImageView::new_default(image.clone()).unwrap();
            Framebuffer::new(
                render_pass.clone(),
                FramebufferCreateInfo {
                    attachments: vec![view],
                    ..Default::default()
                }
            ).unwrap()
        })
        .collect::<Vec<_>>()
}

fn get_pipeline(device: Arc<Device>,
                vertex_shader: Arc<ShaderModule>,
                fragment_shader: Arc<ShaderModule>,
                render_pass: Arc<RenderPass>,
                window: Arc<Window>
) -> Arc<GraphicsPipeline> {
    let vertex_shader = vertex_shader.entry_point("main").unwrap();
    let fragment_shader = fragment_shader.entry_point("main").unwrap();

    let vertex_input_state = Vertex2D::per_vertex()
        .definition(&vertex_shader).unwrap();

    let stages = [
        PipelineShaderStageCreateInfo::new(vertex_shader),
        PipelineShaderStageCreateInfo::new(fragment_shader)
    ];

    let pipeline_layout = PipelineLayout::new(
        device.clone(),
        PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages)
            .into_pipeline_layout_create_info(device.clone()).unwrap()
    ).unwrap();

    let subpass = Subpass::from(render_pass.clone(), 0).unwrap();

    let viewport = Viewport {
        offset: [0.0, 0.0],
        extent: window.inner_size().into(),
        depth_range: 0.0..=1.0
    };

    GraphicsPipeline::new(
        device.clone(),
        None,
        GraphicsPipelineCreateInfo {
            stages: stages.into_iter().collect(),
            vertex_input_state: Some(vertex_input_state),
            input_assembly_state: Some(InputAssemblyState::default()),
            viewport_state: Some(ViewportState {
                viewports: [viewport].into_iter().collect(),
                ..Default::default()
            }),
            rasterization_state: Some(RasterizationState::default()),
            multisample_state: Some(MultisampleState::default()),
            color_blend_state: Some(ColorBlendState::with_attachment_states(
                subpass.num_color_attachments(),
                ColorBlendAttachmentState::default()
            )),
            subpass: Some(subpass.into()),
            ..GraphicsPipelineCreateInfo::layout(pipeline_layout)
        }
    ).unwrap()
}

fn get_command_buffers(
    command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
    queue: Arc<Queue>,
    pipeline: Arc<GraphicsPipeline>,
    framebuffers: Vec<Arc<Framebuffer>>,
    vertex_buffer:
)

#[derive(BufferContents, Vertex)]
#[repr(C)]
struct Vertex2D {
    #[format(R32G32_SFLOAT)]
    position: [f32; 2]
}

mod vertex_shader_module {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "shaders/graphics/shader.vert"
    }
}
mod fragment_shader_module {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "shaders/graphics/shader.frag"
    }
}

impl App {
    pub fn new(event_loop: &EventLoop<()>) -> Self {
        let required_instance_extensions = Surface::required_extensions(&event_loop).unwrap();
        let required_device_extensions = DeviceExtensions {
            khr_swapchain: true,
            ..DeviceExtensions::empty()
        };
        let VulkanPlayground::CommonItems {
            library,
            instance,
            debug_callback,
            queue_family_index,
            device,
            queue,
            memory_allocator,
            descriptor_set_allocator,
            command_buffer_allocator
        } = VulkanPlayground::get_common_vulkan_items(Some(required_instance_extensions),
                                                      Some(required_device_extensions),
                                                      QueueFlags::GRAPHICS);

        let vertex1 = Vertex2D { position: [0.5, -0.5]};
        let vertex2 = Vertex2D { position: [-0.5, 0.25]};
        let vertex3 = Vertex2D { position: [0.0, 0.5]};

        let vertex_buffer = Buffer::from_iter(
            memory_allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::VERTEX_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            vec![vertex1, vertex2, vertex3]
        ).unwrap();




        Self {
            library,
            instance,
            debug_callback,
            queue_family_index,
            device,
            queue,
            render_context: None
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(
            event_loop.create_window(Window::default_attributes()).unwrap()
        );

        let surface = Surface::from_window(self.instance.clone(), window.clone()).unwrap();

        let (mut swapchain, images) = get_swapchain(
            self.device.clone(), surface.clone(), window.clone());

        let render_pass = get_render_pass(self.device.clone(), swapchain.clone());
        let framebuffers = get_framebuffers(images.clone(), render_pass.clone());

        let vertex_shader = vertex_shader_module::load(self.device.clone()).unwrap();
        let fragment_shader = fragment_shader_module::load(self.device.clone()).unwrap();


        let pipeline = get_pipeline(
            self.device.clone(),
            vertex_shader.clone(),
            fragment_shader.clone(),
            render_pass.clone(),
            window.clone()
        );



        self.render_context = Some(RenderContext { window })
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, window_id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            _ => {}
        }
    }
}

fn main() {
    pretty_env_logger::init();

    let event_loop = EventLoop::new().unwrap();
    let mut app = App::new(&event_loop);

    event_loop.run_app(&mut app).unwrap();
}