mod logic;
mod rendering;
mod shader_modules;

use std::env;
use std::collections::BTreeSet;
use std::fs::File;
use std::io::BufReader;
use std::sync::Arc;
use std::time::Instant;
use glam::Vec3;
use log::{info};
use obj::{load_obj, Obj, Vertex};
use vulkano::buffer::{Buffer, BufferCreateInfo, BufferUsage, Subbuffer};
use vulkano::buffer::allocator::{SubbufferAllocator, SubbufferAllocatorCreateInfo};
use vulkano::device::{DeviceExtensions, DeviceFeatures, QueueFlags};
use vulkano::image::view::ImageView;
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryTypeFilter};
use vulkano::pipeline::graphics::viewport::{Viewport};
use vulkano::pipeline::{GraphicsPipeline};
use vulkano::swapchain::{Surface, Swapchain};
use vulkano::sync::GpuFuture;
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode};
use winit::window::{Window, WindowId};
use VulkanPlayground::CommonItems;
use crate::shader_modules::vertex_shader_module::Data;

fn main() {
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App::new(&event_loop);
    event_loop.run_app(&mut app).unwrap();
}

struct App {
    vulkan_items: CommonItems,
    uniform_buffer_allocator: SubbufferAllocator,
    vertex_buffer: Subbuffer<[Vertex]>,
    index_buffer: Subbuffer<[u16]>,
    render_context: Option<RenderContext>,
    logic_items: LogicItems,
}

struct RenderContext {
    window: Arc<Window>,
    swapchain: Arc<Swapchain>,
    color_attachment_image_views: Vec<Arc<ImageView>>,
    depth_attachment_image_view: Arc<ImageView>,
    pipeline: Arc<GraphicsPipeline>,
    viewport: Viewport,
    recreate_swapchain: bool,
    previous_frame_end: Option<Box<dyn GpuFuture>>,
}

struct LogicItems {
    frame_id: i32,
    show_frame_times: bool,
    keys_pressed: BTreeSet<KeyCode>,
    keys_down: BTreeSet<KeyCode>,
    previous_frame_logic_start: Option<Instant>,
    uniform_buffer: Option<Subbuffer<Data>>,
    eye_pos: Vec3,
    eye_horizon: Vec3,
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

        let vulkan_items = VulkanPlayground::get_common_vulkan_items(
            Some(instance_extensions),
            Some(device_extensions),
            Some(device_features),
            QueueFlags::GRAPHICS,
            Some(event_loop)
        );

        let uniform_buffer_allocator = SubbufferAllocator::new(
            vulkan_items.memory_allocator.clone(),
            SubbufferAllocatorCreateInfo {
                buffer_usage: BufferUsage::UNIFORM_BUFFER,
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            }
        );

        let working_dir = env::current_dir().unwrap();
        let obj_path = working_dir.join("resources/bunny_face_normals.obj");
        info!("Reading object at {:?}", obj_path);
        let buf_reader = BufReader::new(File::open(obj_path).unwrap());
        let obj: Obj<Vertex, u16> = load_obj(buf_reader).unwrap();

        let vertex_buffer = Buffer::from_iter(
            vulkan_items.memory_allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::VERTEX_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            obj.vertices
        ).unwrap();

        let index_buffer = Buffer::from_iter(
            vulkan_items.memory_allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::INDEX_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            obj.indices
        ).unwrap();

        let logic_items = LogicItems {
            frame_id: -1,
            show_frame_times: false,
            keys_pressed: BTreeSet::new(),
            keys_down: BTreeSet::new(),
            previous_frame_logic_start: None,
            uniform_buffer: None,
            eye_pos: Vec3::NEG_Z,
            eye_horizon: Vec3::X,
        };

        App {
            vulkan_items,
            uniform_buffer_allocator,
            vertex_buffer,
            index_buffer,
            render_context: None,
            logic_items,
        }
    }
}

impl ApplicationHandler for App {

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.init_render_context(event_loop);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _window_id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(_) => {
                self.render_context.as_mut().unwrap().recreate_swapchain = true;
            }
            WindowEvent::MouseInput {device_id: _, state, button} => {

            }
            WindowEvent::KeyboardInput { device_id: _, event, is_synthetic: _} => {
                self.process_keyboard_input(event);
            }
            WindowEvent::RedrawRequested => {
                let acquire_future = match self.frame_prep() {
                    Some(result) => result,
                    None => return
                };

                let logic_start = Instant::now();
                self.frame_logic();
                let logic_duration = logic_start.elapsed();

                let rendering_start = Instant::now();
                self.frame_render(acquire_future);
                let rendering_duration = rendering_start.elapsed();

                if self.logic_items.show_frame_times {
                    info!("Frame {:5}; logic: {:4.1}, rendering: {:4.1}",
                        self.logic_items.frame_id,
                        logic_duration.as_secs_f32() * 1000.0,
                        rendering_duration.as_secs_f32() * 1000.0,
                    );
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        self.render_context.as_mut().unwrap().window.request_redraw();
    }
}
