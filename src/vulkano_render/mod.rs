pub mod input_handler;
pub mod lyon_render;
pub mod text_render;

use anyhow::{anyhow, Result};
use lazy_static::lazy_static;
use std::sync::Arc;
use vulkano::{
    command_buffer::{AutoCommandBufferBuilder, DynamicState, SubpassContents},
    device::{Device, DeviceExtensions, Queue},
    format::ClearValue,
    framebuffer::{Framebuffer, FramebufferAbstract, RenderPassAbstract},
    image::{view::ImageView, AttachmentImage, ImageAccess, ImageUsage, SwapchainImage},
    instance::{Instance, PhysicalDevice},
    pipeline::viewport::Viewport,
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
    window::{Window, WindowBuilder},
};

use crate::{heart::*, theme};
use input_handler::InputHandler;
use lyon_render::LyonRenderer;
use palette::Pixel;
use parking_lot::Mutex;
use text_render::TextRenderer;
use std::time::Duration;

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

pub fn render(window_builder: WindowBuilder, top_node: Fragment) {
    let event_loop: EventLoop<()> = EventLoop::new();
    let device = VulkanContext::get().device;
    let surface = window_builder.build_vk_surface(&event_loop, device.instance().clone()).unwrap();
    let queue = VulkanContext::get()
        .queues
        .iter()
        .find(|&q| {
            q.family().supports_graphics() && surface.is_supported(q.family()).unwrap_or(false)
        })
        .unwrap()
        .clone();

    let mut dimensions;

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
    let mut previous_frame_end = Some(sync::now(device.clone()).boxed());


    let mut fps_report = FPSReporter::new("gui");

    let mut lyon_renderer = LyonRenderer::new(render_pass.clone());
    let mut text_render = TextRenderer::new(render_pass.clone(), queue.clone());
    let mut input_handler = InputHandler::new();

    let layouter = Arc::new(Mutex::new(Layouter::new(false)));
    let mut evaluator = Evaluator::new(top_node, layouter.clone());

    let mut layouted: Vec<PositionedRenderObject> = vec![];
    let mut recreate_swapchain = false;
    let mut needs_redraw = false;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                *control_flow = ControlFlow::Exit;
            }
            Event::WindowEvent { event: WindowEvent::Resized(_), .. } => {
                recreate_swapchain = true;
            }
            Event::WindowEvent { event: window_event, .. } => {
                if input_handler.handle_input(window_event, layouted.clone(), evaluator.context.clone()) {
                    needs_redraw = true;
                }
            }
            Event::RedrawRequested(_) => {
                needs_redraw = true;
            }
            _ => {}
        }

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

        if needs_redraw {
            let (image_num, suboptimal, acquire_future) =
                match swapchain::acquire_next_image(swapchain.clone(), Some(Duration::from_millis(0))) {
                    Ok(r) => r,
                    Err(AcquireError::OutOfDate) => {
                        recreate_swapchain = true;
                        return;
                    },
                    Err(AcquireError::Timeout) => {
                        return;
                    },
                    Err(e) => panic!("Failed to acquire next image: {:?}", e),
                };
            if suboptimal {
                recreate_swapchain = true;
                return;
            }

            needs_redraw = false;
            previous_frame_end.as_mut().unwrap().cleanup_finished();

            let clear_values =
                vec![theme::BG.into_format().into_raw::<[f32; 4]>().into(), ClearValue::None];
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

            evaluator.update();
            layouted = layouter.lock().do_layout(dimensions.into()).unwrap();

            lyon_renderer.render(
                &mut builder,
                &dynamic_state,
                &dimensions,
                layouted.clone(),
                evaluator.context.clone(),
            );
            text_render.render(
                &mut builder,
                &dynamic_state,
                &dimensions,
                layouted.clone()
            );

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
