use super::{glyph_brush::GlyphBrush, input_handler::InputHandler, lyon::Lyon};
use crate::{
    eval::{
        delta_eval::Evaluator,
        layout::{Layouter, PositionedElement},
    },
    geom::Rect,
    util::fps_report::FPSReporter,
    RenderObject,
    UnevaluatedFragment,
};
use freelist::Idx;

use crate::{
    context::context,
    eval::layout::RenderObjectOrSubPass,
    vulkano_render::{
        primitive_renderer::Renderer,
        subpass_stack::{create_framebuffer, AbstractFramebuffer, AbstractImage, SubPassStack},
        vk_util::VulkanContext,
    },
};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use vulkano::{
    format::Format,
    image::{ImageAccess, ImageUsage, SwapchainImage},
    render_pass::RenderPass,
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

pub fn render(window_builder: WindowBuilder, top_node: UnevaluatedFragment) {
    let mut event_loop: EventLoop<()> = EventLoop::new();
    let VulkanContext { device, queues } = VulkanContext::create().unwrap();
    let surface = window_builder.build_vk_surface(&event_loop, device.instance().clone()).unwrap();
    let queue = queues
        .iter()
        .find(|&q| {
            q.family().supports_graphics() && surface.is_supported(q.family()).unwrap_or(false)
        })
        .unwrap()
        .clone();

    let mut dimensions;

    let caps = surface.capabilities(device.physical_device()).unwrap();
    let format = Format::B8G8R8A8_SRGB;
    let (mut swapchain, images) = {
        let alpha = caps.supported_composite_alpha.iter().next().unwrap();
        dimensions = surface.window().inner_size().into();
        Swapchain::start(device.clone(), surface.clone())
            .usage(ImageUsage {
                color_attachment: true,
                transfer_destination: true,
                ..ImageUsage::none()
            })
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
            .format(format)
            .build()
            .expect("cant create swapchain")
    };

    let render_pass = vulkano::single_pass_renderpass!(device.clone(),
        attachments: {
            intermediary: {
                load: Load,
                store: Store,
                format: swapchain.format(),
                samples: 4,
            },
            depth: {
                load: Load,
                store: Store,
                format: Format::D16_UNORM,
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
    .unwrap();

    let mut framebuffers = window_size_dependent_setup(&images, render_pass.clone());
    let mut previous_frame_end = Some(sync::now(device.clone()).boxed());


    let mut fps_report = FPSReporter::new("gui");

    let mut lyon_renderer = Lyon::new();
    let mut text_render = GlyphBrush::new(queue.clone());
    let mut renderer = Renderer::new(render_pass.clone(), device.clone(), queue.clone());
    let mut input_handler = InputHandler::new();

    let mut layouter = Layouter::new();
    let mut evaluator = Evaluator::new(
        context::VulkanContext { device: device.clone(), queues, render_pass: render_pass.clone() },
        top_node,
        &mut layouter,
    );

    let mut recreate_swapchain = false;
    let mut has_update = true;
    let mut input_render_objects: Vec<(Idx, Option<Rect>)> = Vec::new();

    event_loop.run_return(move |event, _, control_flow| {
        *control_flow = ControlFlow::WaitUntil(Instant::now() + Duration::from_millis(1000 / 70));
        match event {
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                *control_flow = ControlFlow::Exit;
            }
            Event::WindowEvent { event: WindowEvent::Resized(_), .. } => {
                recreate_swapchain = true;
                surface.window().request_redraw();
            }
            Event::WindowEvent { event, .. } => {
                input_handler.enqueue_input(event);
                *control_flow = ControlFlow::Poll;
                return;
            }
            Event::MainEventsCleared => {
                input_handler.handle_input(
                    &input_render_objects[..],
                    &layouter,
                    evaluator.callback_context(&layouter),
                );
                has_update |= evaluator.update(&mut layouter);
                if has_update {
                    surface.window().request_redraw();
                }
            }
            Event::RedrawRequested(_) => {
                previous_frame_end.as_mut().unwrap().cleanup_finished();

                if recreate_swapchain {
                    dimensions = surface.window().inner_size().into();
                    let (new_swapchain, new_images) =
                        match swapchain.recreate().dimensions(dimensions).build() {
                            Ok(r) => r,
                            Err(SwapchainCreationError::UnsupportedDimensions) => {
                                return;
                            }
                            Err(e) => panic!("Failed to recreate swapchain: {:?}", e),
                        };

                    swapchain = new_swapchain;
                    framebuffers = window_size_dependent_setup(&new_images, render_pass.clone());
                    recreate_swapchain = false;
                }

                let (image_num, acquire_fut) = match swapchain::acquire_next_image(
                    swapchain.clone(),
                    Some(Duration::from_millis(0)),
                ) {
                    Ok((image_num, suboptimal, acquire_future)) => {
                        if suboptimal {
                            println!("swapchain suboptimal, need to recreate it");
                            recreate_swapchain = true;
                        }
                        (image_num, acquire_future)
                    }
                    Err(AcquireError::OutOfDate) => {
                        println!("swapchain suboptimal, need to recreate it");
                        recreate_swapchain = true;
                        return;
                    }
                    Err(e) => panic!("Failed to acquire next image: {:?}", e),
                };

                has_update = false;

                let (framebuffer, intermediary_image, depth_image): &(
                    AbstractFramebuffer,
                    AbstractImage,
                    AbstractImage,
                ) = &framebuffers[image_num];

                input_render_objects.clear();

                layouter.do_layout(evaluator.top_node, dimensions.into());

                let mut subpass_stack = SubPassStack::new(
                    format,
                    queue.clone(),
                    render_pass.clone(),
                    framebuffer.clone(),
                    intermediary_image.clone(),
                    depth_image.clone(),
                );

                for (_, obj) in layouter.iter_layouted(evaluator.top_node) {
                    text_render.prerender(&obj);
                }
                let (mut text_state, texture, texture_fut) = text_render.finish(&mut renderer.data);

                for (idx, obj) in layouter.iter_layouted(evaluator.top_node) {
                    subpass_stack.handle(&renderer.data, &obj);
                    lyon_renderer.render(&mut renderer.data, &obj);
                    text_render.render(&obj, &mut renderer.data, &mut text_state);
                    renderer.render(&obj);
                    if let PositionedElement {
                        element: RenderObjectOrSubPass::RenderObject(RenderObject::Input { .. }),
                        ..
                    } = &obj
                    {
                        input_render_objects.push((idx, obj.clipping_rect));
                    }
                }
                subpass_stack.finish(&renderer.data);
                let (
                    vertex_fut,
                    primitive_fut,
                    index_fut,
                    descriptor_set,
                    vertex_buffer,
                    index_buffer,
                ) = renderer.finish(texture);

                let after_frame_callbacks = std::mem::take(&mut evaluator.after_frame_callbacks);
                let callback_context = evaluator.callback_context(&layouter);
                let command_buffer = subpass_stack.render(
                    &callback_context,
                    |builder, viewport, dimensions, offset, start, end| {
                        renderer.render_part(
                            builder,
                            descriptor_set.clone(),
                            viewport,
                            dimensions,
                            offset,
                            start,
                            end,
                        )
                    },
                    vertex_buffer,
                    index_buffer,
                );
                fps_report.frame();

                let future = previous_frame_end
                    .take()
                    .unwrap()
                    .join(
                        texture_fut
                            .map(|v| Box::new(v) as Box<dyn GpuFuture>)
                            .unwrap_or_else(|| Box::new(sync::now(device.clone())) as _),
                    )
                    .join(index_fut)
                    .join(vertex_fut)
                    .join(primitive_fut)
                    .join(acquire_fut)
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

                for callback in after_frame_callbacks {
                    callback(&callback_context);
                }
            }
            _e => {}
        }
    });
}

/// This method is called once during initialization, then again whenever the
/// window is resized
fn window_size_dependent_setup(
    images: &[Arc<SwapchainImage<Window>>],
    render_pass: Arc<RenderPass>,
) -> Vec<(AbstractFramebuffer, AbstractImage, AbstractImage)> {
    let dimensions = images[0].dimensions();
    images
        .iter()
        .map(|image| {
            let fb = create_framebuffer(
                [dimensions.width(), dimensions.height()],
                render_pass.clone(),
                image.format(),
                Some(image.clone()),
            );
            (fb.0, fb.3, fb.4)
        })
        .collect::<Vec<_>>()
}
