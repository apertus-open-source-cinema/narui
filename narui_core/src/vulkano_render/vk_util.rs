use anyhow::{anyhow, Result};
use std::{
    ffi::CStr,
    os::raw::{c_char, c_void},
};

use std::sync::Arc;
use vulkano::{
    device::{
        physical::{PhysicalDevice, PhysicalDeviceType},
        Device,
        DeviceCreationError,
        DeviceExtensions,
        Queue,
    },
    instance::{
        debug::*,
        loader::{FunctionPointers, Loader},
        Instance,
        InstanceExtensions,
    },
    Version,
};


#[derive(Clone)]
pub struct VulkanContext {
    pub device: Arc<Device>,
    pub queues: Vec<Arc<Queue>>,
}

#[cfg(not(target_os = "macos"))]
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


#[cfg(target_os = "macos")]
impl VulkanContext {
    pub fn create() -> Result<Self> {
        let required_extensions = vulkano_win::required_extensions();
        let extensions = InstanceExtensions {
            khr_surface: true,
            mvk_macos_surface: true,
            ext_debug_utils: true,
            ext_debug_report: true,
            ..required_extensions
        };

        struct AshMoltenLoader {
            staticFn: ash_molten_version::vk::StaticFn,
        }
        unsafe impl Loader for AshMoltenLoader {
            fn get_instance_proc_addr(
                &self,
                instance: ash_vulkano_version::vk::Instance,
                name: *const c_char,
            ) -> *const c_void {
                let inner_result = unsafe {
                    self.staticFn.get_instance_proc_addr(std::mem::transmute(instance), name)
                };
                if let Some(result) = inner_result {
                    result as *const c_void
                } else {
                    0 as *const c_void
                }
            }
        }

        let function_pointers: FunctionPointers<Box<dyn Loader + Send + Sync>> =
            FunctionPointers::new(Box::new(AshMoltenLoader {
                staticFn: ash_molten::load().static_fn().clone(),
            }));
        let instance =
            Instance::with_loader(function_pointers, None, Version::V1_2, &extensions, None)?;
        std::mem::forget(DebugCallback::new(
            &instance,
            MessageSeverity { error: true, warning: true, information: true, verbose: true },
            MessageType::all(),
            |msg| log::info!("{}: {}", msg.layer_prefix.unwrap_or("unknown"), msg.description),
        ));

        let mut devices_vec: Vec<_> = PhysicalDevice::enumerate(&instance).collect();
        devices_vec.sort_unstable_by_key(|dev| {
            if dev.properties().device_type == PhysicalDeviceType::Cpu {
                1
            } else {
                0
            }
        });
        let mut error: Option<DeviceCreationError> = None;
        let (device, queues) = devices_vec
            .into_iter()
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
                match Device::new(
                    physical,
                    physical.supported_features(),
                    &device_ext,
                    queue_family,
                ) {
                    Ok(v) => Some(v),
                    Err(e) => {
                        error = Some(e.clone());
                        None
                    }
                }
            })
            .ok_or_else(|| anyhow!("No physical device found. Last error was: {:?}", error))?;

        Ok(Self { device, queues: queues.collect() })
    }
}
