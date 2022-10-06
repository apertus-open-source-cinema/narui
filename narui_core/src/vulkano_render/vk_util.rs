use anyhow::{anyhow, Result};
use std::sync::Arc;
use vulkano::{
    device::{
        physical::{PhysicalDevice, PhysicalDeviceType},
        Device,
        DeviceCreateInfo,
        DeviceExtensions,
        Queue,
        QueueCreateInfo,
    },
    instance::{
        debug::{
            DebugUtilsMessageSeverity,
            DebugUtilsMessageType,
            DebugUtilsMessenger,
            DebugUtilsMessengerCreateInfo,
        },
        Instance,
        InstanceCreateInfo,
        InstanceExtensions,
    },
};
use vulkano_maybe_molten::NewMaybeMolten;


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

        let instance = Instance::new_maybe_molten(InstanceCreateInfo {
            enabled_extensions: extensions,
            ..Default::default()
        })?;

        // Safety: callback must not make any calls to the Vulkan API
        unsafe {
            std::mem::forget(DebugUtilsMessenger::new(
                instance.clone(),
                DebugUtilsMessengerCreateInfo {
                    message_severity: DebugUtilsMessageSeverity::all(),
                    message_type: DebugUtilsMessageType::all(),

                    ..DebugUtilsMessengerCreateInfo::user_callback(Arc::new(|msg| {
                        log::info!("{}: {}", msg.layer_prefix.unwrap_or("unknown"), msg.description)
                    }))
                },
            ));
        }

        let mut devices_vec: Vec<_> = PhysicalDevice::enumerate(&instance).collect();
        devices_vec.sort_unstable_by_key(|dev| {
            if dev.properties().device_type == PhysicalDeviceType::Cpu {
                1
            } else {
                0
            }
        });
        let (device, queues) = devices_vec
            .into_iter()
            .find_map(|physical| {
                let queue_family = physical.queue_families().map(QueueCreateInfo::family).collect(); // All queues have the same priority
                let device_ext = DeviceExtensions {
                    khr_swapchain: true,
                    khr_storage_buffer_storage_class: true,
                    khr_8bit_storage: true,
                    // Comment in if you need shader printf
                    // khr_shader_non_semantic_info: true,
                    ..DeviceExtensions::none()
                };
                Device::new(
                    physical,
                    DeviceCreateInfo {
                        enabled_features: physical.supported_features().clone(),
                        enabled_extensions: device_ext,
                        queue_create_infos: queue_family,
                        ..Default::default()
                    },
                )
                .ok()
            })
            .ok_or_else(|| anyhow!("No physical device found"))?;

        Ok(Self { device, queues: queues.collect() })
    }
}
