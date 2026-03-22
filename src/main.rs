use vulkano::device::{Device, DeviceCreateInfo, QueueCreateInfo, QueueFlags};
use vulkano::device::physical::PhysicalDeviceType;
use vulkano::instance::{Instance, InstanceCreateInfo, InstanceExtensions};
use vulkano::VulkanLibrary;
use VulkanPlayground::get_debug_callback;

const DEFAULT_INSTANCE_EXTENSIONS: InstanceExtensions = InstanceExtensions {
    ext_debug_utils: true,
    ..InstanceExtensions::empty()
};
const LAYERS: [&str; 1] = ["VK_LAYER_KHRONOS_validation"];

fn main() {
    let library = VulkanLibrary::new().expect("No local Vulkan library/dll");

    let mut library_layers = library.layer_properties().unwrap();
    LAYERS.iter().for_each(|layer| {
        library_layers.find(|l| {l.name() == *layer})
            .expect(format!("Layer {} not available in library", *layer).as_str());
    });

    let instance = Instance::new(
        library.clone(),
        InstanceCreateInfo {
            enabled_layers: LAYERS.iter().map(|l| {l.to_string()}).collect::<Vec<_>>(),
            enabled_extensions: DEFAULT_INSTANCE_EXTENSIONS,
            ..Default::default()
        }
    ).expect("Failed to create instance");

    let debug_callback = get_debug_callback(instance.clone());

    let physical_device = instance
        .enumerate_physical_devices().unwrap()
        .min_by_key(|physical_device| match physical_device.properties().device_type {
            PhysicalDeviceType::DiscreteGpu => 0,
            PhysicalDeviceType::IntegratedGpu => 1,
            _ => 2,
        }).unwrap();

    let queue_family_index = physical_device
        .queue_family_properties().iter().enumerate()
        .position(|(_, queue_family_properties)| {
            queue_family_properties.queue_flags.contains(QueueFlags::GRAPHICS)
        })
        .expect("No queue with appropriate support available") as u32;

    let (device, mut queues) = Device::new(
        physical_device.clone(),
        DeviceCreateInfo {
            queue_create_infos: vec![QueueCreateInfo {
                queue_family_index,
                ..Default::default()
            }],
            ..Default::default()
        }
    ).expect("Failed to create device");

    mod compute_shader_module {
        vulkano_shaders::shader!{
            ty: "compute",
            path: r"shaders\compute.glsl",
        }
    }
    let shader_module = compute_shader_module::load(device.clone()).expect("Failed to create shader module");
}