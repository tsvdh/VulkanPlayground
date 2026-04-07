mod logic;
mod rendering;

use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::Instant;
use log::{info};
use vulkano::buffer::{Buffer, BufferContents, BufferCreateInfo, BufferUsage, Subbuffer};
use vulkano::device::{DeviceExtensions, DeviceFeatures, QueueFlags};
use vulkano::image::{Image};
use vulkano::image::view::ImageView;
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryTypeFilter};
use vulkano::pipeline::graphics::viewport::{Viewport};
use vulkano::pipeline::{GraphicsPipeline};
use vulkano::pipeline::graphics::vertex_input::Vertex;
use vulkano::swapchain::{Surface, Swapchain};
use vulkano::sync::GpuFuture;
use winit::application::ApplicationHandler;
use winit::event::{WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode};
use winit::window::{Window, WindowId};
use VulkanPlayground::CommonItems;

fn main() {
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App::new(&event_loop);
    event_loop.run_app(&mut app).unwrap();
}

struct App {
    vulkan_items: CommonItems,
    vertex_buffer: Subbuffer<[BasicVertex]>,
    render_context: Option<RenderContext>,
    logic_items: LogicItems,
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

struct LogicItems {
    frame_id: i32,
    show_frame_times: bool,
    keys_pressed: BTreeSet<KeyCode>,
    keys_down: BTreeSet<KeyCode>,
    previous_frame_logic_start: Option<Instant>,
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

        let vertex1 = BasicVertex { position: [0.0, -0.5]};
        let vertex2 = BasicVertex { position: [0.5, 0.0]};
        let vertex3 = BasicVertex { position: [-0.5, 0.0]};

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
            vec![vertex1, vertex2, vertex3]
        ).unwrap();

        let logic_items = LogicItems {
            frame_id: -1,
            show_frame_times: true,
            keys_pressed: BTreeSet::new(),
            keys_down: BTreeSet::new(),
            previous_frame_logic_start: None,
        };

        App {
            vulkan_items,
            vertex_buffer,
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
                self.handle_keyboard_input(event);
            }
            WindowEvent::RedrawRequested => {
                self.logic_items.frame_id += 1;

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
                    info!("Frame {:5}; logic: {:4.1}, total: {:4.1}",
                        self.logic_items.frame_id,
                        logic_duration.as_secs_f32() * 1000.0,
                        rendering_duration.as_secs_f32() * 1000.0,);
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        self.render_context.as_mut().unwrap().window.request_redraw();
    }
}

fn make_image_views(images: &[Arc<Image>]) -> Vec<Arc<ImageView>> {
    images.iter().map(|image| {
        ImageView::new_default(image.clone()).unwrap()
    }).collect()
}