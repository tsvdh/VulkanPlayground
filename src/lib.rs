use std::sync::Arc;
use log::{debug, error, info, warn};
use vulkano::command_buffer::allocator::{StandardCommandBufferAllocator, StandardCommandBufferAllocatorCreateInfo};
use vulkano::descriptor_set::allocator::StandardDescriptorSetAllocator;
use vulkano::device::{Device, DeviceCreateInfo, Queue, QueueCreateInfo, QueueFlags};
use vulkano::device::physical::{PhysicalDevice, PhysicalDeviceType};
use vulkano::instance::debug::{DebugUtilsMessageSeverity, DebugUtilsMessageType, DebugUtilsMessenger, DebugUtilsMessengerCallback, DebugUtilsMessengerCreateInfo};
use vulkano::instance::{Instance, InstanceCreateInfo, InstanceExtensions};
use vulkano::memory::allocator::StandardMemoryAllocator;
use vulkano::VulkanLibrary;

const EXTENSIONS: InstanceExtensions = InstanceExtensions {
    ext_debug_utils: true,
    ..InstanceExtensions::empty()
};
const LAYERS: [&str; 1] = ["VK_LAYER_KHRONOS_validation"];

pub struct CommonItems {
    pub library: Arc<VulkanLibrary>,
    pub instance: Arc<Instance>,
    pub debug_callback: DebugUtilsMessenger,
    pub physical_device: Arc<PhysicalDevice>,
    pub queue_family_index: u32,
    pub device: Arc<Device>,
    pub queue: Arc<Queue>,
    pub memory_allocator: Arc<StandardMemoryAllocator>,
    pub descriptor_set_allocator: Arc<StandardDescriptorSetAllocator>,
    pub command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
}

pub fn get_debug_callback(instance: Arc<Instance>) -> DebugUtilsMessenger {
    unsafe {
        DebugUtilsMessenger::new(
            instance.clone(),
            DebugUtilsMessengerCreateInfo {
                message_severity: DebugUtilsMessageSeverity::ERROR
                    | DebugUtilsMessageSeverity::WARNING
                    | DebugUtilsMessageSeverity::INFO
                    | DebugUtilsMessageSeverity::VERBOSE,
                message_type: DebugUtilsMessageType::GENERAL
                    | DebugUtilsMessageType::PERFORMANCE
                    | DebugUtilsMessageType::VALIDATION,
                ..DebugUtilsMessengerCreateInfo::user_callback(DebugUtilsMessengerCallback::new(
                    |message_severity,
                     message_type,
                     callback_data| {
                        if message_severity.intersects(DebugUtilsMessageSeverity::ERROR) {
                            error!("({:?}) {}", message_type, callback_data.message);
                        } else if message_severity.intersects(DebugUtilsMessageSeverity::WARNING) {
                            warn!("({:?}) {}", message_type, callback_data.message);
                        } else if message_severity.intersects(DebugUtilsMessageSeverity::INFO) {
                            info!("({:?}) {}", message_type, callback_data.message);
                        } else {
                            debug!("({:?}) {}", message_type, callback_data.message);
                        }
                    }
                ))

            }
        ).expect("Failed to create debug callback")
    }
}

pub fn get_common_items() -> CommonItems {
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
            enabled_extensions: EXTENSIONS,
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
        .expect("No queue with compute support available") as u32;

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

    let queue = queues.next().unwrap();

    let memory_allocator = Arc::new(StandardMemoryAllocator::new_default(
        device.clone())
    );
    let descriptor_set_allocator = Arc::new(StandardDescriptorSetAllocator::new(
        device.clone(), Default::default())
    );
    let command_buffer_allocator = Arc::new(StandardCommandBufferAllocator::new(
        device.clone(), StandardCommandBufferAllocatorCreateInfo::default()
    ));

    CommonItems{
        library,
        instance,
        debug_callback,
        physical_device,
        queue_family_index,
        device,
        queue,
        memory_allocator,
        descriptor_set_allocator,
        command_buffer_allocator
    }
}