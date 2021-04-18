pub mod lyon_render;

use crate::{
    api::{RenderObject, Widget},
    layout::{do_layout, PositionedRenderObject},
};
use anyhow::{anyhow, Context, Result};
use lazy_static::lazy_static;
use lyon::{
    lyon_algorithms::path::{
        builder::PathBuilder,
        geom::{point, Translation},
    },
    lyon_tessellation::{BuffersBuilder, FillOptions, FillTessellator, FillVertex, VertexBuffers},
    tessellation::{path::Path, FillVertexConstructor},
};
use std::sync::Arc;

use crate::{fps_report::FPSReporter, vulkano_render::lyon_render::LyonRenderer};
use stretch::geometry::Size;
use vulkano::{
    buffer::{BufferUsage, BufferView, CpuAccessibleBuffer},
    command_buffer::{AutoCommandBufferBuilder, DynamicState, SubpassContents},
    descriptor::descriptor_set::PersistentDescriptorSet,
    device::{Device, DeviceExtensions, Queue},
    format::{ClearValue, Format, R8Unorm},
    framebuffer::{Framebuffer, FramebufferAbstract, RenderPassAbstract, Subpass},
    image::{view::ImageView, AttachmentImage, ImageAccess, ImageUsage, SwapchainImage},
    instance::{Instance, PhysicalDevice},
    pipeline::{raster::PolygonMode::Point, viewport::Viewport, GraphicsPipeline},
    swapchain,
    swapchain::{
        AcquireError,
        ColorSpace,
        FullscreenExclusive,
        PresentMode,
        SurfaceTransform,
        Swapchain,
        SwapchainCreationError,
    },
    sync,
    sync::{FlushError, GpuFuture},
};
use vulkano_win::VkSurfaceBuild;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    platform::{run_return::EventLoopExtRunReturn, unix::EventLoopExtUnix},
    window::{Window, WindowBuilder},
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
        let instance = Instance::new(None, &required_extensions, None)?;
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

pub fn render(top_node: Widget) {
    let mut event_loop: EventLoop<()> = EventLoopExtUnix::new_any_thread();
    let device = VulkanContext::get().device;
    let surface = WindowBuilder::new()
        .with_title("axiom converter vulkan output")
        .build_vk_surface(&event_loop, device.instance().clone())
        .unwrap();
    let queue = VulkanContext::get()
        .queues
        .iter()
        .find(|&q| {
            q.family().supports_graphics() && surface.is_supported(q.family()).unwrap_or(false)
        })
        .unwrap()
        .clone();

    let mut dimensions = [1u32, 1];

    let (mut swapchain, images) = {
        let caps = surface.capabilities(device.physical_device()).unwrap();
        let alpha = caps.supported_composite_alpha.iter().next().unwrap();
        let format = caps.supported_formats[0].0;
        dimensions = surface.window().inner_size().into();

        Swapchain::new(
            device.clone(),
            surface.clone(),
            caps.min_image_count,
            format,
            dimensions,
            1,
            ImageUsage::color_attachment(),
            &queue,
            SurfaceTransform::Identity,
            alpha,
            PresentMode::Fifo,
            FullscreenExclusive::Default,
            true,
            ColorSpace::SrgbNonLinear,
        )
        .unwrap()
    };

    let render_pass = Arc::new(
        vulkano::single_pass_renderpass!(device.clone(),
            attachments: {
                intermediary: {
                    load: Clear,
                    store: DontCare,
                    format: swapchain.format(),
                    samples: 4,     // This has to match the image definition.
                },
                color: {
                    load: DontCare,
                    store: Store,
                    format: swapchain.format(),
                    samples: 1,
                }
            },
            pass: {
                color: [intermediary],
                depth_stencil: {},
                resolve: [color],
            }
        )
        .unwrap(),
    );

    let mut dynamic_state = DynamicState {
        line_width: None,
        viewports: None,
        scissors: None,
        compare_mask: None,
        write_mask: None,
        reference: None,
    };
    let mut framebuffers =
        window_size_dependent_setup(&images, render_pass.clone(), &mut dynamic_state);
    let mut recreate_swapchain = false;
    let mut previous_frame_end = Some(sync::now(device.clone()).boxed());

    let mut fps_report = FPSReporter::new("gui");

    event_loop.run_return(move |event, _, control_flow| match event {
        Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
            *control_flow = ControlFlow::Exit;
        }
        Event::WindowEvent { event: WindowEvent::Resized(_), .. } => {
            recreate_swapchain = true;
        }
        Event::RedrawEventsCleared => {
            previous_frame_end.as_mut().unwrap().cleanup_finished();
            if recreate_swapchain {
                dimensions = surface.window().inner_size().into();
                let (new_swapchain, new_images) =
                    match swapchain.recreate_with_dimensions(dimensions) {
                        Ok(r) => r,
                        Err(SwapchainCreationError::UnsupportedDimensions) => return,
                        Err(e) => panic!("Failed to recreate swapchain: {:?}", e),
                    };

                swapchain = new_swapchain;
                framebuffers = window_size_dependent_setup(
                    &new_images,
                    render_pass.clone(),
                    &mut dynamic_state,
                );
                recreate_swapchain = false;
            }

            let (image_num, suboptimal, acquire_future) =
                match swapchain::acquire_next_image(swapchain.clone(), None) {
                    Ok(r) => r,
                    Err(AcquireError::OutOfDate) => {
                        recreate_swapchain = true;
                        return;
                    }
                    Err(e) => panic!("Failed to acquire next image: {:?}", e),
                };

            if suboptimal {
                recreate_swapchain = true;
            }

            let clear_values = vec![[0.0, 0.0, 0.0, 1.0].into(), ClearValue::None];
            let mut builder =
                AutoCommandBufferBuilder::primary_one_time_submit(device.clone(), queue.family())
                    .unwrap();
            builder
                .begin_render_pass(
                    framebuffers[image_num].clone(),
                    SubpassContents::Inline,
                    clear_values,
                )
                .unwrap();

            let layouted = do_layout(
                top_node.clone(),
                Size { width: dimensions[0] as f32, height: dimensions[1] as f32 },
            )
            .unwrap();

            let lyon_renderer = LyonRenderer::new(render_pass.clone());
            lyon_renderer.render(&mut builder, &dynamic_state, &dimensions, layouted);

            builder.end_render_pass().unwrap();
            let command_buffer = builder.build().unwrap();
            fps_report.frame();

            let future = previous_frame_end
                .take()
                .unwrap()
                .join(acquire_future)
                .then_execute(queue.clone(), command_buffer)
                .unwrap()
                .then_swapchain_present(queue.clone(), swapchain.clone(), image_num)
                .then_signal_fence_and_flush();

            match future {
                Ok(future) => {
                    previous_frame_end = Some(future.boxed());
                }
                Err(FlushError::OutOfDate) => {
                    recreate_swapchain = true;
                    previous_frame_end = Some(sync::now(device.clone()).boxed());
                }
                Err(e) => {
                    println!("Failed to flush future: {:?}", e);
                    previous_frame_end = Some(sync::now(device.clone()).boxed());
                }
            }
        }
        _ => {}
    });
}

/// This method is called once during initialization, then again whenever the
/// window is resized
fn window_size_dependent_setup(
    images: &[Arc<SwapchainImage<Window>>],
    render_pass: Arc<dyn RenderPassAbstract + Send + Sync>,
    dynamic_state: &mut DynamicState,
) -> Vec<Arc<dyn FramebufferAbstract + Send + Sync>> {
    let dimensions = images[0].dimensions();

    let viewport = Viewport {
        origin: [0.0, 0.0],
        dimensions: [dimensions.width() as f32, dimensions.height() as f32],
        depth_range: 0.0..1.0,
    };
    dynamic_state.viewports = Some(vec![viewport]);

    images
        .iter()
        .map(|image| {
            let intermediary = ImageView::new(
                AttachmentImage::transient_multisampled(
                    render_pass.device().clone(),
                    [dimensions.width(), dimensions.height()],
                    4,
                    image.format(),
                )
                .unwrap(),
            )
            .unwrap();
            let view = ImageView::new(image.clone()).unwrap();
            Arc::new(
                Framebuffer::start(render_pass.clone())
                    .add(intermediary)
                    .unwrap()
                    .add(view)
                    .unwrap()
                    .build()
                    .unwrap(),
            ) as Arc<dyn FramebufferAbstract + Send + Sync>
        })
        .collect::<Vec<_>>()
}
