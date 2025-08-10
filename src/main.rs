use std::sync::Arc;
use std::time::Instant;
use log::{debug, error, info, warn};
use vulkano::buffer::{Buffer, BufferCreateInfo, BufferUsage};
use vulkano::command_buffer::allocator::{StandardCommandBufferAllocator, StandardCommandBufferAllocatorCreateInfo};
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage};
use vulkano::descriptor_set::allocator::StandardDescriptorSetAllocator;
use vulkano::descriptor_set::{DescriptorSet, WriteDescriptorSet};
use vulkano::device::{Device, DeviceCreateInfo, QueueCreateInfo, QueueFlags};
use vulkano::device::physical::{PhysicalDeviceType};
use vulkano::instance::{Instance, InstanceCreateInfo, InstanceExtensions};
use vulkano::instance::debug::{DebugUtilsMessageSeverity, DebugUtilsMessageType, DebugUtilsMessenger, DebugUtilsMessengerCallback, DebugUtilsMessengerCreateInfo};
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryTypeFilter, StandardMemoryAllocator};
use vulkano::pipeline::{ComputePipeline, Pipeline, PipelineBindPoint, PipelineLayout, PipelineShaderStageCreateInfo};
use vulkano::pipeline::compute::ComputePipelineCreateInfo;
use vulkano::pipeline::layout::PipelineDescriptorSetLayoutCreateInfo;
use vulkano::{sync, VulkanLibrary};
use vulkano::sync::GpuFuture;

const EXTENSIONS: InstanceExtensions = InstanceExtensions {
    ext_debug_utils: true,
    ..InstanceExtensions::empty()
};

const LAYERS: [&str; 1] = ["VK_LAYER_KHRONOS_validation"];

const NUM_VALUES: u32 = 67_108_864;

fn main() {
    pretty_env_logger::init();

    let gpu_setup_start = Instant::now();

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

    let content = 0..NUM_VALUES;
    let buffer = Buffer::from_iter(
        memory_allocator.clone(),
        BufferCreateInfo {
            usage: BufferUsage::STORAGE_BUFFER,
            ..Default::default()
        },
        AllocationCreateInfo {
            memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
            ..Default::default()
        },
        content
    ).expect("Failed to create buffer");

    mod cs {
        vulkano_shaders::shader!{
            ty: "compute",
            path: r"shaders\compute.glsl"
        }
    }
    let shader = cs::load(device.clone()).expect("Failed to create shader module");

    let cs = shader.entry_point("main").unwrap();
    let stage = PipelineShaderStageCreateInfo::new(cs);
    let layout = PipelineLayout::new(
        device.clone(),
        PipelineDescriptorSetLayoutCreateInfo::from_stages([&stage])
            .into_pipeline_layout_create_info(device.clone()).unwrap()
    ).unwrap();

    let compute_pipeline = ComputePipeline::new(
        device.clone(), None,
        ComputePipelineCreateInfo::stage_layout(stage, layout)
    ).expect("Failed to create compute pipeline");

    let descriptor_set_allocator = Arc::new(StandardDescriptorSetAllocator::new(
        device.clone(), Default::default()));
    let pipeline_layout = compute_pipeline.layout();
    let descriptor_set_layouts = pipeline_layout.set_layouts();

    let descriptor_set_layout_index = 0;
    let descriptor_set_layout = descriptor_set_layouts.get(descriptor_set_layout_index).unwrap();
    let descriptor_set = DescriptorSet::new(
        descriptor_set_allocator.clone(),
        descriptor_set_layout.clone(),
        [WriteDescriptorSet::buffer(0, buffer.clone())],
        []
    ).unwrap();

    let command_buffer_allocator = Arc::new(StandardCommandBufferAllocator::new(
        device.clone(),
        StandardCommandBufferAllocatorCreateInfo::default()
    ));

    let mut command_buffer_builder = AutoCommandBufferBuilder::primary(
        command_buffer_allocator.clone(),
        queue_family_index,
        CommandBufferUsage::OneTimeSubmit
    ).unwrap();

    let work_group_counts = [NUM_VALUES / 64, 1, 1];

    unsafe {
        command_buffer_builder
            .bind_pipeline_compute(compute_pipeline.clone()).unwrap()
            .bind_descriptor_sets(
                PipelineBindPoint::Compute,
                pipeline_layout.clone(),
                descriptor_set_layout_index as u32,
                descriptor_set.clone()
            ).unwrap()
            .dispatch(work_group_counts).unwrap();
    }
    let command_buffer = command_buffer_builder.build().unwrap();

    info!("GPU setup took: {}ms", gpu_setup_start.elapsed().as_millis());
    let gpu_execution_start = Instant::now();

    let future = sync::now(device.clone())
        .then_execute(queue.clone(), command_buffer.clone()).unwrap()
        .then_signal_fence_and_flush().unwrap();

    future.wait(None).unwrap();

    info!("GPU execution took: {}ms", gpu_execution_start.elapsed().as_millis());

    let buffer_content = buffer.read().unwrap();
    for (i, item) in buffer_content.iter().enumerate() {
        assert_eq!(*item, (i * 12) as u32);
    }

    let cpu_setup_start = Instant::now();
    let mut cpu_content = (0..NUM_VALUES).collect::<Vec<_>>();
    info!("CPU setup took: {}ms", cpu_setup_start.elapsed().as_millis());

    let cpu_execution_start = Instant::now();
    for i in 0..cpu_content.len() {
        cpu_content[i] *= 12;
    }
    info!("CPU execution took: {}ms", cpu_execution_start.elapsed().as_millis());
}