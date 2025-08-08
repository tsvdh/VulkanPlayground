use std::sync::Arc;
use log::{debug, error, info, warn};
use vulkano::{sync, VulkanLibrary};
use vulkano::buffer::{Buffer, BufferCreateInfo, BufferUsage};
use vulkano::command_buffer::allocator::{StandardCommandBufferAllocator, StandardCommandBufferAllocatorCreateInfo};
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage, CopyBufferInfo};
use vulkano::device::{Device, DeviceCreateInfo, QueueCreateInfo, QueueFlags};
use vulkano::instance::{Instance, InstanceCreateInfo, InstanceExtensions};
use vulkano::instance::debug::{DebugUtilsMessageSeverity, DebugUtilsMessageType, DebugUtilsMessenger, DebugUtilsMessengerCallback, DebugUtilsMessengerCreateInfo};
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryTypeFilter, StandardMemoryAllocator};
use vulkano::sync::GpuFuture;

const EXTENSIONS: InstanceExtensions = InstanceExtensions {
    ext_debug_utils: true,
    ..InstanceExtensions::empty()
};

const LAYERS: [&str; 1] = ["VK_LAYER_KHRONOS_validation"];

fn main() {
    pretty_env_logger::init();

    let library = VulkanLibrary::new().expect("No local Vulkan library/dll");

    let mut library_layers = library.layer_properties().unwrap();
    LAYERS.iter().for_each(|layer| {
        library_layers.find(|l| {l.name() == *layer})
            .expect(format!("Layer {} not available in library", *layer).as_str());
    });

    let instance = Instance::new(
        library,
        InstanceCreateInfo {
            enabled_layers: LAYERS.iter().map(|l| {l.to_string()}).collect::<Vec<_>>(),
            enabled_extensions: EXTENSIONS,
            ..Default::default()
        }
    ).expect("Failed to create instance");


    let _debug_callback = unsafe {
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
        )
    };

    let physical_device = instance
        .enumerate_physical_devices().expect("Could not create physical device")
        .next().expect("No devices available");

    let queue_family_index = physical_device
        .queue_family_properties().iter().enumerate()
        .position(|(_, queue_family_properties)| {
            queue_family_properties.queue_flags.contains(QueueFlags::GRAPHICS)
        })
        .expect("No queue with graphics support available") as u32;

    let (device, mut queues) = Device::new(
        physical_device,
        DeviceCreateInfo {
            queue_create_infos: vec![QueueCreateInfo {
                queue_family_index,
                ..Default::default()
            }],
            ..Default::default()
        }
    ).expect("Failed to create device");

    let queue = queues.next().unwrap();

    let memory_allocator = Arc::new(StandardMemoryAllocator::new_default(device.clone()));

    let source_content: Vec<i32> = (0..32).collect();
    let source_buffer = Buffer::from_iter(
        memory_allocator.clone(),
        BufferCreateInfo {
            usage: BufferUsage::TRANSFER_SRC,
            ..Default::default()
        },
        AllocationCreateInfo {
            memory_type_filter: MemoryTypeFilter::PREFER_HOST
                | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
            ..Default::default()
        },
        source_content
    ).expect("Failed to create source buffer");

    let destination_content: Vec<i32> = (0..32).map(|_| 0).collect();
    let destination_buffer = Buffer::from_iter(
        memory_allocator.clone(),
        BufferCreateInfo {
            usage: BufferUsage::TRANSFER_DST,
            ..Default::default()
        },
        AllocationCreateInfo {
            memory_type_filter: MemoryTypeFilter::PREFER_HOST
                | MemoryTypeFilter::HOST_RANDOM_ACCESS,
            ..Default::default()
        },
        destination_content
    ).expect("Failed to create destination buffer");

    let command_buffer_allocator = Arc::new(StandardCommandBufferAllocator::new(
        device.clone(),
        StandardCommandBufferAllocatorCreateInfo::default()
    ));

    let mut command_buffer_builder = AutoCommandBufferBuilder::primary(
        command_buffer_allocator,
        queue_family_index,
        CommandBufferUsage::OneTimeSubmit
    ).unwrap();

    command_buffer_builder.copy_buffer(CopyBufferInfo::buffers(source_buffer.clone(), destination_buffer.clone())).unwrap();
    let command_buffer = command_buffer_builder.build().unwrap();

    println!("before: {:?}", destination_buffer.read().unwrap().to_vec());

    let future = sync::now(device.clone())
        .then_execute(queue.clone(), command_buffer).unwrap()
        .then_signal_fence_and_flush().unwrap();

    future.wait(None).unwrap();
    println!("after: {:?}", destination_buffer.read().unwrap().to_vec());

}