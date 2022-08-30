use super::{glyph_brush::GlyphBrush, input_handler::InputHandler, lyon::Lyon};
use crate::{
    eval::{
        delta_eval::Evaluator,
        layout::{Layouter, Physical, PhysicalPositionedElement, ScaleFactor},
    },
    geom::Rect,
    util::fps_report::FPSReporter,
    RenderObject,
    UnevaluatedFragment,
    Vec2,
};
use freelist::Idx;

use crate::{
    context::context,
    eval::layout::RenderObjectOrSubPass,
    vulkano_render::{
        frame_pacing::FramePacer,
        primitive_renderer::Renderer,
        subpass_stack::{create_framebuffer, AbstractFramebuffer, AbstractImage, SubPassStack},
        vk_util::VulkanContext,
    },
};
use std::{
    convert::TryFrom,
    sync::Arc,
    time::{Duration, Instant},
};
use vulkano::{
    format::Format,
    image::{ImageAccess, ImageUsage, SampleCount, SwapchainImage},
    render_pass::RenderPass,
    swapchain::{
        self,
        AcquireError,
        PresentMode,
        Swapchain,
        SwapchainCreateInfo,
        SwapchainCreationError,
    },
    sync,
    sync::{FenceSignalFuture, FlushError, GpuFuture},
};
use vulkano_win::VkSurfaceBuild;
use wayland_client::{
    protocol::{wl_display::WlDisplay, wl_surface::WlSurface},
    GlobalManager,
};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    platform::{
        run_return::EventLoopExtRunReturn,
        unix::{EventLoopWindowTargetExtUnix, WindowExtUnix},
    },
    window::{Window, WindowBuilder},
};

pub fn render(window_builder: WindowBuilder, top_node: UnevaluatedFragment) {
    let mut event_loop: EventLoop<()> = EventLoop::new();
    let VulkanContext { device, queues } = VulkanContext::create().unwrap();
    let surface = window_builder.build_vk_surface(&event_loop, device.instance().clone()).unwrap();
    let queue = queues
        .iter()
        .find(|&q| {
            let fam = q.family();
            fam.supports_graphics() && fam.supports_surface(&surface).unwrap_or(false)
        })
        .unwrap()
        .clone();

    let mut dimensions;

    let caps = device.physical_device().surface_capabilities(&surface, Default::default()).unwrap();
    let format = Format::B8G8R8A8_SRGB;
    let (mut swapchain, images) = {
        let alpha = caps.supported_composite_alpha.iter().next().unwrap();
        dimensions = surface.window().inner_size().into();
        Swapchain::new(
            device.clone(),
            surface.clone(),
            SwapchainCreateInfo {
                image_usage: ImageUsage {
                    color_attachment: true,
                    transfer_dst: true,
                    ..ImageUsage::none()
                },
                min_image_count: caps.min_image_count,
                composite_alpha: alpha,
                image_extent: dimensions,
                image_format: Some(format),
                present_mode: PresentMode::Mailbox,
                ..Default::default()
            },
        )
        .expect("cant create swapchain")
    };

    let render_pass = vulkano::single_pass_renderpass!(device.clone(),
        attachments: {
            intermediary: {
                load: Load,
                store: Store,
                format: swapchain.image_format(),
                samples: SampleCount::Sample4,
            },
            depth: {
                load: Load,
                store: Store,
                format: Format::D16_UNORM,
                samples: SampleCount::Sample4,
            },
            color: {
                load: DontCare,
                store: Store,
                format: swapchain.image_format(),
                samples: SampleCount::Sample1,
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
    let mut previous_frame_end = Some(sync::now(device.clone()).boxed_send_sync());


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
    let mut input_render_objects: Vec<(Idx, Option<Physical<Rect>>)> = Vec::new();


    let wl_display = event_loop.wayland_display().unwrap();
    let wl_surface = surface.window().wayland_surface().unwrap();
    let wl_display =
        unsafe { wayland_client::Display::from_external_display(wl_display as *mut _) };
    let wl_surface =
        unsafe { wayland_client::Proxy::<WlSurface>::from_c_ptr(wl_surface as *mut _) };
    let mut wl_event_queue = wl_display.create_event_queue();
    let attached_display = (*wl_display).clone().attach(wl_event_queue.token());
    use wayland_protocols::presentation_time::client::{wp_presentation, wp_presentation_feedback};


    let global_manager = GlobalManager::new(&attached_display);
    wl_event_queue.sync_roundtrip(&mut (), |_, _, _| unreachable!()).unwrap();
    let wp_presentation =
        global_manager.instantiate_exact::<wp_presentation::WpPresentation>(1).unwrap();

    let clockid = Arc::new(std::cell::Cell::new(None));
    let clockid_wl = clockid.clone();
    wp_presentation.quick_assign(move |_, event, _| match event {
        wp_presentation::Event::ClockId { clk_id } => {
            clockid_wl.set(Some(clk_id));
        }
        _ => {}
    });
    wl_event_queue.sync_roundtrip(&mut (), |_, _, _| {}).unwrap();
    let clockid = clockid.get().unwrap();


    let frame_pacer = Arc::new(parking_lot::Mutex::new(FramePacer::new(clockid as _)));
    let frame_pacer_wl = frame_pacer.clone();
    let frame_pacer_timing = frame_pacer.clone();

    let (fence_tx, fence_rx) = std::sync::mpsc::channel::<Arc<FenceSignalFuture<_>>>();

    let thread = std::thread::spawn(move || {
        while let Ok(fence) = fence_rx.recv() {
            fence.wait(None);
            frame_pacer_timing.lock().swapchain_present();
        }
    });

    fn constrain<A, B, T>(t: T) -> T
    where
        T: for<'a> FnMut(A, B, wayland_client::DispatchData<'a>),
    {
        t
    }
    let handler = constrain(move |_, event: wp_presentation_feedback::Event, _| match event {
        wp_presentation_feedback::Event::Presented {
            seq_hi,
            seq_lo,
            tv_sec_lo,
            tv_nsec,
            refresh,
            ..
        } => {
            let seq = ((seq_hi as i64) << 32) | (seq_lo as i64);
            let time = ((tv_sec_lo as i64) * 1_000_000_000) + tv_nsec as i64;
            let refresh = refresh as i64;
            frame_pacer_wl.lock().present_time(refresh, time, seq);
        }
        wp_presentation_feedback::Event::Discarded => {
            frame_pacer_wl.lock().discarded_frame();
        }
        _ => {}
    });
    // dbg!(globals.list());


    event_loop.run_return(move |event, _, control_flow| {
        *control_flow = ControlFlow::WaitUntil(Instant::now() + Duration::from_micros(500));
        let scale_factor = ScaleFactor(surface.window().scale_factor() as f32);
        // *control_flow = ControlFlow::Poll;
        wl_event_queue.sync_roundtrip(&mut (), |_, _, _| {}).unwrap();

        match event {
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                *control_flow = ControlFlow::Exit;
            }
            Event::WindowEvent { event: WindowEvent::Resized(_), .. } => {
                recreate_swapchain = true;
                surface.window().request_redraw();
            }
            Event::WindowEvent { event, .. } => {
                //                println!("got input");
                input_handler.enqueue_input(event);
                *control_flow = ControlFlow::Poll;
            }
            Event::MainEventsCleared => {
                input_handler.handle_input(
                    &input_render_objects[..],
                    &layouter,
                    evaluator.callback_context(&layouter, &scale_factor),
                    scale_factor,
                );
                has_update |= evaluator.update(&mut layouter);
                if has_update {
                    let time = frame_pacer.lock().want_redraw();
                    if let Some(time) = time {
                        *control_flow = ControlFlow::WaitUntil(time);
                    } else {
                        surface.window().request_redraw();
                    }
                    //                    println!("requesting redraw");
                }
            }
            Event::RedrawRequested(_) => {
                //                println!("redraw requested after {}ms",
                // t.duration_since(start).as_secs_f64() * 1000.0);
                frame_pacer.lock().render_loop_begin();
                previous_frame_end.as_mut().unwrap().cleanup_finished();

                if recreate_swapchain {
                    dimensions = surface.window().inner_size().into();
                    let (new_swapchain, new_images) =
                        match swapchain.recreate(SwapchainCreateInfo {
                            image_extent: dimensions,
                            ..swapchain.create_info()
                        }) {
                            Ok(r) => r,
                            Err(SwapchainCreationError::ImageExtentNotSupported { .. }) => {
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
                    Some(Duration::from_nanos(1)),
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
                    Err(AcquireError::Timeout) => {
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

                layouter.do_layout(evaluator.top_node, Vec2::from(dimensions) / scale_factor.0);

                let mut subpass_stack = SubPassStack::new(
                    format,
                    queue.clone(),
                    render_pass.clone(),
                    framebuffer.clone(),
                    intermediary_image.clone(),
                    depth_image.clone(),
                );

                for (_, obj) in layouter.iter_layouted_physical(evaluator.top_node, scale_factor) {
                    text_render.prerender(&obj, scale_factor);
                }
                let (mut text_state, texture, texture_fut) = text_render.finish(&mut renderer.data);

                for (idx, obj) in layouter.iter_layouted_physical(evaluator.top_node, scale_factor)
                {
                    subpass_stack.handle(&renderer.data, &obj);
                    lyon_renderer.render(&mut renderer.data, &obj, scale_factor);
                    text_render.render(&obj, &mut renderer.data, &mut text_state);
                    renderer.render(&obj, scale_factor);
                    if let PhysicalPositionedElement {
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
                let callback_context = evaluator.callback_context(&layouter, &scale_factor);

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

                // TODO(robin): is this the right place?
                wp_presentation.feedback(&wl_surface.clone().into()).quick_assign(handler.clone());

                let future = previous_frame_end
                    .take()
                    .unwrap()
                    .join(
                        texture_fut
                            .map(|v| Box::new(v) as Box<dyn GpuFuture + Sync + Send>)
                            .unwrap_or_else(|| {
                                Box::new(sync::now(device.clone()))
                                    as Box<dyn GpuFuture + Send + Sync>
                            }),
                    )
                    .join(index_fut)
                    .join(vertex_fut)
                    .join(primitive_fut)
                    .join(acquire_fut)
                    .then_execute(queue.clone(), command_buffer)
                    .unwrap()
                    .then_swapchain_present(queue.clone(), swapchain.clone(), image_num)
                    .then_signal_fence_and_flush();

                let future = match future {
                    Ok(future) => {
                        let future = Arc::new(future);
                        previous_frame_end = Some(future.clone().boxed_send_sync());
                        frame_pacer.lock().render_loop_end();
                        fence_tx.send(future).unwrap();
                    }
                    Err(FlushError::OutOfDate) => {
                        recreate_swapchain = true;
                        previous_frame_end = Some(sync::now(device.clone()).boxed_send_sync());
                    }
                    Err(e) => {
                        println!("Failed to flush future: {:?}", e);
                        previous_frame_end = Some(sync::now(device.clone()).boxed_send_sync());
                    }
                };

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
