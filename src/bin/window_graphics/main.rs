mod logic;
mod rendering;
mod shader_modules;
mod ui;

use std::env;
use std::collections::{BTreeSet, VecDeque};
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io::BufReader;
use std::sync::Arc;
use std::time::{Duration, Instant};
use egui_winit_vulkano::{Gui};
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
use vulkano::swapchain::{PresentFuture, Surface, Swapchain};
use vulkano::sync::future::FenceSignalFuture;
use vulkano::sync::GpuFuture;
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode};
use winit::window::{Window, WindowId};
use VulkanPlayground::CommonItems;
use crate::shader_modules::vertex_shader_module::VertexData;
use crate::shader_modules::fragment_shader_module::FragmentData;

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
    egui: Option<Gui>,
    durations: Durations
}

struct RenderContext {
    window: Arc<Window>,
    swapchain: Arc<Swapchain>,
    color_attachment_image_views: Vec<Arc<ImageView>>,
    depth_attachment_image_view: Arc<ImageView>,
    pipeline: Arc<GraphicsPipeline>,
    viewport: Viewport,
    recreate_swapchain: bool,
    previous_frame_end: Option<FenceSignalFuture<PresentFuture<Box<dyn GpuFuture>>>>,
}

struct LogicItems {
    frame_id: i32,
    show_frame_times: bool,
    min_frame_duration: Duration,
    keys_pressed: BTreeSet<KeyCode>,
    keys_down: BTreeSet<KeyCode>,
    frame_start_moments: VecDeque<Instant>,
    vertex_shader_uniform_buffer: Option<Subbuffer<VertexData>>,
    fragment_shader_uniform_buffer: Option<Subbuffer<FragmentData>>,
    eye_pos: Vec3,
    eye_horizon: Vec3,
    light_pos: Vec3,
}

struct Durations {
    ui_duration: Option<Duration>,
    logic_duration: Option<Duration>,
    rendering_start: Option<Instant>,
    render_duration: Option<Duration>,
    total_duration: Option<Duration>,
}

impl Durations {

    fn empty() -> Self {
        Durations {
            ui_duration: None,
            logic_duration: None,
            rendering_start: None,
            render_duration: None,
            total_duration: None,
        }
    }

    fn display_duration(duration: Option<Duration>) -> String {
        match duration {
            None => {format!("{:>4}", "--")}
            Some(duration) => {format!("{:4.1}", duration.as_secs_f32() * 1000.0)}
        }
    }
}

impl Display for Durations {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "ui: {}, logic: {}, rendering: {}, total: {}",
               Self::display_duration(self.ui_duration),
               Self::display_duration(self.logic_duration),
               Self::display_duration(self.render_duration),
               Self::display_duration(self.total_duration),
        )
    }
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

        let min_frame_duration = Duration::from_secs_f32(1.0 / 60.0);

        let mut frame_start_moments: VecDeque<Instant> = VecDeque::new();
        let now = Instant::now();
        frame_start_moments.push_back(now - min_frame_duration);
        frame_start_moments.push_back(now);

        let logic_items = LogicItems {
            frame_id: 0,
            show_frame_times: true,
            min_frame_duration,
            keys_pressed: BTreeSet::new(),
            keys_down: BTreeSet::new(),
            frame_start_moments,
            vertex_shader_uniform_buffer: None,
            fragment_shader_uniform_buffer: None,
            eye_pos: Vec3::new(0.0, 0.0, -1.5),
            eye_horizon: Vec3::X,
            light_pos: Vec3::new(0.0, 10.0, 0.0),
        };

        let durations = Durations::empty();

        App {
            vulkan_items,
            uniform_buffer_allocator,
            vertex_buffer,
            index_buffer,
            render_context: None,
            logic_items,
            egui: None,
            durations
        }
    }
}

impl ApplicationHandler for App {

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attributes = Window::default_attributes()
            .with_title("VulkanPlayground")
            .with_inner_size(PhysicalSize::new(1280, 960));
        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        self.init_render_context(window.clone());
        self.init_egui(event_loop);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _window_id: WindowId, event: WindowEvent) {
        if self.egui.as_mut().unwrap().update(&event) {
            return;
        }

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
                if !self.new_frame_start() {
                    return
                }

                if self.logic_items.show_frame_times {
                    info!("Frame {:5} | {}", self.logic_items.frame_id, self.durations)
                }
                self.durations = Durations::empty();
                self.logic_items.frame_id += 1;

                // new frame start

                let acquire_future = match self.frame_rendering_prep() {
                    None => return,
                    Some(result) => result,
                };

                let ui_start = Instant::now();
                self.build_ui();
                self.durations.ui_duration = Some(ui_start.elapsed());

                let logic_start = Instant::now();
                self.frame_logic();
                self.durations.logic_duration = Some(logic_start.elapsed());

                self.durations.rendering_start = Some(Instant::now());
                self.frame_render(acquire_future);
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        self.render_context.as_mut().unwrap().window.request_redraw();
    }
}
