pub mod input_handler;
pub mod lyon_render;
pub mod raw_render;
pub mod text_render;
pub mod util;

pub use util::VulkanContext;

use crate::{heart::*, raw_render::RawRenderer, theme, util::FPSReporter};
use hashbrown::HashSet;
use input_handler::InputHandler;
use lyon_render::LyonRenderer;
use palette::Pixel;
use rutter_layout::Idx;
use std::{
    collections::VecDeque,
    sync::Arc,
    time::{Duration, Instant},
};
use text_render::TextRenderer;
use vulkano::{
    command_buffer::{
        AutoCommandBufferBuilder,
        CommandBufferUsage::OneTimeSubmit,
        DynamicState,
        SubpassContents,
    },
    device::DeviceOwned,
    format::{ClearValue, Format},
    image::{
        view::ImageView,
        AttachmentImage,
        ImageAccess,
        ImageUsage,
        SampleCount::Sample4,
        SwapchainImage,
    },
    pipeline::viewport::Viewport,
    render_pass::{Framebuffer, FramebufferAbstract, RenderPass},
    swapchain,
    swapchain::{AcquireError, PresentMode, Swapchain, SwapchainCreationError},
    sync,
    sync::{FlushError, GpuFuture},
};
use vulkano_win::VkSurfaceBuild;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    platform::run_return::EventLoopExtRunReturn,
    window::{Window, WindowBuilder},
};

pub fn render(window_builder: WindowBuilder, top_node: Fragment) {
    let mut event_loop: EventLoop<()> = EventLoop::new();
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

    let caps = surface.capabilities(device.physical_device()).unwrap();
    let (mut swapchain, images) = {
        let alpha = caps.supported_composite_alpha.iter().next().unwrap();
        dimensions = surface.window().inner_size().into();
        Swapchain::start(device.clone(), surface.clone())
            .usage(ImageUsage::color_attachment())
            .num_images(caps.min_image_count)
            .composite_alpha(alpha)
            .dimensions(dimensions)
            .present_mode(
                if std::env::var("NARUI_PRESENT_MODE_MAILBOX").is_ok() {
                    PresentMode::Mailbox
                } else {
                    PresentMode::Fifo
                },
            )
            .format(Format::B8G8R8A8Srgb)
            .build()
            .expect("cant create swapchain")
    };

    let render_pass = Arc::new(
        vulkano::single_pass_renderpass!(device.clone(),
            attachments: {
                intermediary: {
                    load: Clear,
                    store: DontCare,
                    format: swapchain.format(),
                    samples: 4,
                },
                depth: {
                    load: Clear,
                    store: Store,
                    format: Format::D16Unorm,
                    samples: 4,
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
                depth_stencil: {depth},
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
    let mut raw_render = RawRenderer::new(render_pass.clone());
    let mut input_handler = InputHandler::new();

    let mut layouter = Layouter::new();
    let mut evaluator = Evaluator::new(top_node, &mut layouter);

    let mut recreate_swapchain = false;
    let mut acquired_images = VecDeque::with_capacity(caps.min_image_count as usize);
    let mut has_update = true;
    let mut input_render_objects: HashSet<Idx, ahash::RandomState> = HashSet::default();

    event_loop.run_return(move |event, _, control_flow| {
        *control_flow = ControlFlow::WaitUntil(Instant::now() + Duration::from_millis(1000 / 70));
        match event {
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                *control_flow = ControlFlow::Exit;
            }
            Event::WindowEvent { event: WindowEvent::Resized(_), .. } => {
                recreate_swapchain = true;
            }
            Event::WindowEvent { event, .. } => {
                input_handler.enqueue_input(event);
                *control_flow = ControlFlow::Poll;
                return;
            }
            Event::MainEventsCleared => {
                input_handler.handle_input(
                    &input_render_objects,
                    &layouter,
                    evaluator.callback_context(&layouter),
                );
                has_update |= evaluator.update(&mut layouter);
                if has_update && (acquired_images.len() >= (caps.min_image_count) as usize - 1) {
                    surface.window().request_redraw();
                }
            }
            Event::RedrawRequested(_) => {
                has_update = false;
                let (image_num, acquire_future) = acquired_images.pop_front().unwrap();

                previous_frame_end.as_mut().unwrap().cleanup_finished();

                let clear_values =
                    vec![theme::BG.into_raw::<[f32; 4]>().into(), 1f32.into(), ClearValue::None];
                let mut builder = AutoCommandBufferBuilder::primary(
                    device.clone(),
                    queue.family(),
                    OneTimeSubmit,
                )
                .unwrap();
                let framebuffer = <Arc<_> as Clone>::clone(&framebuffers[image_num]);
                builder
                    .begin_render_pass(framebuffer, SubpassContents::Inline, clear_values)
                    .unwrap();

                layouter.do_layout(evaluator.top_node, dimensions.into());

                input_render_objects.clear();
                let mut lyon_state = lyon_renderer.begin();
                for (idx, obj) in layouter.iter_layouted(evaluator.top_node) {
                    raw_render.render(&mut builder, &dynamic_state, &dimensions, &obj);
                    lyon_renderer.render(&mut lyon_state, &obj);
                    text_render.render(&obj);
                    if let PositionedRenderObject {
                        render_object: RenderObject::Input { .. },
                        ..
                    } = &obj
                    {
                        input_render_objects.insert(idx);
                    }
                }
                lyon_renderer.finish(lyon_state, &mut builder, &dynamic_state, &dimensions);
                text_render.finish(&mut builder, &dynamic_state, &dimensions);

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

                let after_frame_callbacks = std::mem::take(&mut evaluator.after_frame_callbacks);
                let context = evaluator.callback_context(&layouter);
                for callback in after_frame_callbacks {
                    callback(&context);
                }
            }
            _e => {}
        }

        if recreate_swapchain {
            dimensions = surface.window().inner_size().into();
            let (new_swapchain, new_images) =
                match swapchain.recreate().dimensions(dimensions).build() {
                    Ok(r) => r,
                    Err(SwapchainCreationError::UnsupportedDimensions) => return,
                    Err(e) => panic!("Failed to recreate swapchain: {:?}", e),
                };

            swapchain = new_swapchain;
            framebuffers =
                window_size_dependent_setup(&new_images, render_pass.clone(), &mut dynamic_state);
            recreate_swapchain = false;
            acquired_images.clear();
        }


        // here we fill our FIFO of surfaces we can draw to - as eagerly as we can
        match swapchain::acquire_next_image(swapchain.clone(), Some(Duration::from_millis(0))) {
            Ok((image_num, suboptimal, acquire_future)) => {
                if suboptimal {
                    recreate_swapchain = true;
                    return;
                }
                acquired_images.push_back((image_num, acquire_future));
            }
            Err(AcquireError::OutOfDate) => {
                recreate_swapchain = true;
            }
            Err(AcquireError::Timeout) => {}
            Err(e) => panic!("Failed to acquire next image: {:?}", e),
        };
    });
}

/// This method is called once during initialization, then again whenever the
/// window is resized
fn window_size_dependent_setup(
    images: &[Arc<SwapchainImage<Window>>],
    render_pass: Arc<RenderPass>,
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
                    Sample4,
                    image.format(),
                )
                .unwrap(),
            )
            .unwrap();
            let view = ImageView::new(image.clone()).unwrap();
            let depth_buffer = ImageView::new(
                AttachmentImage::multisampled_with_usage(
                    render_pass.device().clone(),
                    [dimensions.width(), dimensions.height()],
                    Sample4,
                    Format::D16Unorm,
                    ImageUsage {
                        transient_attachment: true,
                        input_attachment: true,
                        ..ImageUsage::none()
                    },
                )
                .unwrap(),
            )
            .unwrap();
            Arc::new(
                Framebuffer::start(render_pass.clone())
                    .add(intermediary)
                    .unwrap()
                    .add(depth_buffer)
                    .unwrap()
                    .add(view)
                    .unwrap()
                    .build()
                    .unwrap(),
            ) as Arc<dyn FramebufferAbstract + Send + Sync>
        })
        .collect::<Vec<_>>()
}
