use anyhow::{anyhow, Result};
use lazy_static::lazy_static;
use std::sync::Arc;
use vulkano::{
    device::{physical::PhysicalDevice, Device, DeviceExtensions, Queue},
    instance::Instance,
    Version,
};


#[derive(Clone)]
pub struct VulkanContext {
    pub device: Arc<Device>,
    pub queues: Vec<Arc<Queue>>,
}
lazy_static! {
    pub static ref VULKAN_CONTEXT: VulkanContext = VulkanContext::create().unwrap();
}
impl VulkanContext {
    pub fn create() -> Result<Self> {
        let required_extensions = vulkano_win::required_extensions();
        let instance = Instance::new(None, Version::V1_2, &required_extensions, None)?;
        let physical = PhysicalDevice::enumerate(&instance)
            .next()
            .ok_or_else(|| anyhow!("No physical device found"))?;
        let queue_family = physical.queue_families().map(|qf| (qf, 0.5)); // All queues have the same priority
        let device_ext = DeviceExtensions {
            khr_swapchain: true,
            khr_storage_buffer_storage_class: true,
            khr_8bit_storage: true,
            ..DeviceExtensions::none()
        };
        let (device, queues) =
            Device::new(physical, physical.supported_features(), &device_ext, queue_family)?;
        Ok(Self { device, queues: queues.collect() })
    }
    pub fn get() -> Self { VULKAN_CONTEXT.clone() }
}
