use anyhow::{anyhow, Result};

use std::sync::Arc;
use vulkano::{
    device::{physical::PhysicalDevice, Device, DeviceExtensions, Queue},
    instance::{debug::*, Instance, InstanceExtensions},
    Version,
};


#[derive(Clone)]
pub struct VulkanContext {
    pub device: Arc<Device>,
    pub queues: Vec<Arc<Queue>>,
}

impl VulkanContext {
    pub fn create() -> Result<Self> {
        let required_extensions = vulkano_win::required_extensions();
        let extensions = InstanceExtensions {
            ext_debug_utils: true,
            ext_debug_report: true,
            ..required_extensions
        };
        let instance = Instance::new(None, Version::V1_2, &extensions, None)?;
        std::mem::forget(DebugCallback::new(
            &instance,
            MessageSeverity { error: true, warning: true, information: true, verbose: true },
            MessageType::all(),
            |msg| log::info!("{}: {}", msg.layer_prefix.unwrap_or("unknown"), msg.description),
        ));

        let (device, queues) = PhysicalDevice::enumerate(&instance)
            .find_map(|physical| {
                let queue_family = physical.queue_families().map(|qf| (qf, 0.5)); // All queues have the same priority
                let device_ext = DeviceExtensions {
                    khr_swapchain: true,
                    khr_storage_buffer_storage_class: true,
                    khr_8bit_storage: true,
                    // Comment in if you need shader printf
                    // khr_shader_non_semantic_info: true,
                    ..(*physical.required_extensions())
                };
                Device::new(physical, physical.supported_features(), &device_ext, queue_family).ok()
            })
            .ok_or_else(|| anyhow!("No physical device found"))?;

        Ok(Self { device, queues: queues.collect() })
    }
}
