use crate::{
    eval::layout::{Physical, PhysicalPositionedElement, RenderObjectOrSubPass},
    vulkano_render::primitive_renderer::RenderData,
    CallbackContext,
    Rect,
    RenderFnInner,
    RenderObject,
    SubPassRenderFunction,
    Vec2,
};
use derivative::Derivative;
use std::{fmt::Formatter, sync::Arc};
use vulkano::{
    buffer::{BufferAccess, BufferAccessObject, TypedBufferAccess},
    command_buffer::{
        AutoCommandBufferBuilder,
        ClearColorImageInfo,
        ClearDepthStencilImageInfo,
        CommandBufferUsage::OneTimeSubmit,
        PrimaryAutoCommandBuffer,
        RenderPassBeginInfo,
        SubpassContents,
    },
    device::{DeviceOwned, Queue},
    format::Format,
    image::{
        view::ImageView,
        AttachmentImage,
        ImageAccess,
        ImageUsage,
        ImageViewAbstract,
        SampleCount,
    },
    pipeline::graphics::viewport::Viewport,
    render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass},
};

// render function, color, depth, absolute layout rect, z_index
type Finisher = (SubPassRenderFunction, AbstractImageView, AbstractImageView, Physical<Rect>, u32);

struct SubPassData {
    fb: AbstractFramebuffer,
    intermediary: AbstractImage,
    depth_image: AbstractImage,
    // only None at the first "virtual" (toplevel) subpass
    depth: Option<AbstractImageView>,
    color: Option<AbstractImageView>,
    rect: Physical<Rect>,
    first: usize,
    first_use: bool,
    finishers: Vec<Finisher>,
}

fn opaque_fmt<T>(o: &Option<T>, fmt: &mut Formatter) -> std::fmt::Result {
    if o.is_some() {
        write!(fmt, "Some(..)")
    } else {
        write!(fmt, "None")
    }
}

#[derive(Derivative)]
#[derivative(Debug)]
pub enum SubPassRenderCommand {
    RenderPrimitive {
        #[derivative(Debug = "ignore")]
        target: AbstractFramebuffer,
        offset: Physical<Vec2>,
        start: usize,
        end: usize, // exclusive
        #[derivative(Debug(format_with = "opaque_fmt"))]
        to_clear: Option<(AbstractImage, AbstractImage)>,
    },
    Raw {
        #[derivative(Debug = "ignore")]
        target: AbstractFramebuffer,
        #[derivative(Debug = "ignore")]
        fun: Arc<RenderFnInner>,
        z_index: u32,
        rect: Physical<Rect>,
        clipping_rect: Option<Physical<Rect>>,
        #[derivative(Debug(format_with = "opaque_fmt"))]
        to_clear: Option<(AbstractImage, AbstractImage)>,
    },
    ResolveOrFinish {
        #[derivative(Debug = "ignore")]
        resolve: SubPassRenderFunction,
        #[derivative(Debug = "ignore")]
        depth: AbstractImageView,
        #[derivative(Debug = "ignore")]
        color: AbstractImageView,
        #[derivative(Debug = "ignore")]
        to: AbstractFramebuffer,
        abs_rect: Physical<Rect>,
        rect: Physical<Rect>,
        z_index: u32,
        #[derivative(Debug(format_with = "opaque_fmt"))]
        to_clear: Option<(AbstractImage, AbstractImage)>,
    },
}

pub type AbstractImageView = Arc<dyn ImageViewAbstract + Send + Sync>;
pub type AbstractFramebuffer = Arc<Framebuffer>;
pub type AbstractImage = Arc<dyn ImageAccess + Send + Sync>;

pub struct SubPassStack {
    queue: Arc<Queue>,
    format: Format,
    render_pass: Arc<RenderPass>,
    stack: Vec<SubPassData>,
    pub(crate) render_commands: Vec<SubPassRenderCommand>,
}

impl SubPassStack {
    pub fn new(
        format: Format,
        queue: Arc<Queue>,
        render_pass: Arc<RenderPass>,
        toplevel_fb: AbstractFramebuffer,
        toplevel_intermediary: AbstractImage,
        toplevel_depth: AbstractImage,
    ) -> Self {
        let stack = vec![SubPassData {
            fb: toplevel_fb,
            intermediary: toplevel_intermediary,
            depth_image: toplevel_depth,
            depth: None,
            color: None,
            rect: Physical::new(Rect::zero()),
            first: 0,
            first_use: true,
            finishers: vec![],
        }];
        Self { format, queue, render_pass, stack, render_commands: vec![] }
    }
}

pub fn create_framebuffer<I: ImageAccess + Send + Sync + std::fmt::Debug + 'static>(
    size: [u32; 2],
    render_pass: Arc<RenderPass>,
    format: Format,
    target_image: Option<Arc<I>>,
) -> (AbstractFramebuffer, AbstractImageView, AbstractImageView, AbstractImage, AbstractImage) {
    let intermediary_image = AttachmentImage::multisampled_with_usage(
        render_pass.device().clone(),
        size,
        SampleCount::Sample4,
        format,
        ImageUsage { color_attachment: true, transfer_dst: true, ..ImageUsage::none() },
    )
    .unwrap();
    let intermediary = ImageView::new_default(intermediary_image.clone()).unwrap();

    let target = match target_image {
        Some(i) => ImageView::new_default(i).unwrap() as AbstractImageView,
        None => ImageView::new_default(
            AttachmentImage::with_usage(
                render_pass.device().clone(),
                size,
                format,
                ImageUsage {
                    color_attachment: true,
                    sampled: true,
                    transfer_dst: true,
                    ..ImageUsage::none()
                },
            )
            .unwrap(),
        )
        .unwrap() as AbstractImageView,
    };

    let depth_image = AttachmentImage::multisampled_with_usage(
        render_pass.device().clone(),
        size,
        SampleCount::Sample4,
        Format::D16_UNORM,
        ImageUsage {
            depth_stencil_attachment: true,
            sampled: true,
            transfer_dst: true,
            ..ImageUsage::none()
        },
    )
    .unwrap();

    let depth = ImageView::new_default(depth_image.clone()).unwrap();

    (
        Framebuffer::new(
            render_pass,
            FramebufferCreateInfo {
                attachments: vec![intermediary, depth.clone(), target.clone()],
                ..Default::default()
            },
        )
        .unwrap() as AbstractFramebuffer,
        depth,
        target,
        intermediary_image,
        depth_image,
    )
}

impl SubPassStack {
    fn push_render_primitive(&mut self, data: &RenderData) {
        let pass = self.stack.last().unwrap();
        let rect = pass.rect;
        let first = pass.first;
        if pass.first != data.indices.len() {
            self.push_command(data, |target, to_clear| SubPassRenderCommand::RenderPrimitive {
                offset: rect.map(|rect| rect.pos),
                target,
                start: first,
                end: data.indices.len(),
                to_clear,
            });
        }
    }

    fn push_command<
        F: FnOnce(AbstractFramebuffer, Option<(AbstractImage, AbstractImage)>) -> SubPassRenderCommand,
    >(
        &mut self,
        data: &RenderData,
        fun: F,
    ) {
        let pass = self.stack.last_mut().unwrap();
        let (to_clear, target) = (
            if pass.first_use {
                Some((pass.intermediary.clone(), pass.depth_image.clone()))
            } else {
                None
            },
            pass.fb.clone(),
        );

        pass.first = data.indices.len();
        pass.first_use = false;
        self.render_commands.push(fun(target, to_clear));
    }

    fn push_finishers(
        &mut self,
        data: &RenderData,
        offset: Physical<Vec2>,
        finishers: &[Finisher],
    ) {
        for (finish, color, depth, rect, z_index) in finishers {
            self.push_command(data, |target, to_clear| SubPassRenderCommand::ResolveOrFinish {
                resolve: finish.clone(),
                color: color.clone(),
                depth: depth.clone(),
                to: target,
                abs_rect: *rect,
                rect: rect.map(|rect| rect.minus_position(offset.unwrap_physical())),
                z_index: *z_index,
                to_clear,
            });
        }
    }

    pub fn handle(&mut self, data: &RenderData, obj: &PhysicalPositionedElement) {
        match &obj.element {
            RenderObjectOrSubPass::SubPassPush => {
                self.push_render_primitive(data);
                let (fb, depth, color, intermediary, depth_image) =
                    create_framebuffer::<AttachmentImage>(
                        obj.rect.unwrap_physical().size.pixels(),
                        self.render_pass.clone(),
                        self.format,
                        None,
                    );
                self.stack.push(SubPassData {
                    fb,
                    intermediary,
                    depth: Some(depth),
                    color: Some(color),
                    depth_image,
                    rect: obj.rect,
                    first: data.indices.len(),
                    first_use: true,
                    finishers: vec![],
                });
            }
            RenderObjectOrSubPass::SubPassPop(setup) => {
                self.push_render_primitive(data);
                let pass = self.stack.pop().unwrap();

                let offset = self.stack.last().unwrap().rect.map(|rect| rect.pos);
                self.push_finishers(data, offset, &pass.finishers[..]);

                let color = pass.color.unwrap().clone();
                let depth = pass.depth.unwrap().clone();
                self.push_command(data, |target, to_clear| SubPassRenderCommand::ResolveOrFinish {
                    resolve: setup.resolve.clone(),
                    color: color.clone(),
                    depth: depth.clone(),
                    to: target,
                    abs_rect: obj.rect,
                    rect: obj.rect.map(|rect| rect.minus_position(offset.unwrap_physical())),
                    z_index: obj.z_index,
                    to_clear,
                });

                if let Some((finish, target)) = &setup.finish {
                    let target = match target {
                        Some(offset) => {
                            let pos = self.stack.len() - 1 - offset;
                            &mut self.stack[pos]
                        }
                        None => self.stack.last_mut().unwrap(),
                    };
                    target.finishers.push((finish.clone(), color, depth, obj.rect, obj.z_index));
                }
            }
            RenderObjectOrSubPass::RenderObject(RenderObject::Raw { render_fn }) => {
                self.push_render_primitive(data);
                self.push_command(data, |target, to_clear| SubPassRenderCommand::Raw {
                    target,
                    fun: render_fn.clone(),
                    z_index: obj.z_index,
                    rect: obj.rect,
                    clipping_rect: obj.clipping_rect,
                    to_clear,
                });
            }
            _ => {}
        }
    }

    pub fn finish(&mut self, data: &RenderData) {
        self.push_render_primitive(data);
        let finishers = std::mem::take(&mut self.stack.last_mut().unwrap().finishers);
        self.push_finishers(data, Physical::new(Vec2::zero()), &finishers[..]);
    }
    pub fn render<F, B: BufferAccess + 'static>(
        &mut self,
        callback_context: &CallbackContext,
        primitive_renderer: F,
        vertex_buffer: Arc<B>,
        index_buffer: Arc<impl BufferAccess + TypedBufferAccess<Content = [u32]> + 'static>,
    ) -> PrimaryAutoCommandBuffer
    where
        Arc<B>: BufferAccessObject,
        F: Fn(
            &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
            &Viewport,
            &[u32; 2],
            Physical<Vec2>,
            u64,
            u64,
        ),
    {
        log::trace!("collected render commands {:?}", self.render_commands);

        let mut builder = AutoCommandBufferBuilder::primary(
            self.render_pass.device().clone(),
            self.queue.family(),
            OneTimeSubmit,
        )
        .unwrap();

        let mut viewport;
        let mut dimensions;

        macro_rules! push_render_pass {
            ($builder:ident, $target:ident, $viewport:ident, $dimensions:ident, $to_clear:ident $(, $secondary:expr)?) => {
                let clear_values = vec![None, None, None, None];
                // let clear_values = vec![[0., 0., 1., 1.].into(), 1f32.into(), ClearValue::None];
                let dims = $target.attachments()[0].image().dimensions();
                $dimensions = [dims.width(), dims.height()];
                $viewport = Viewport {
                    origin: [0.0, 0.0],
                    dimensions: [dimensions[0] as f32, dimensions[1] as f32],
                    depth_range: 0.0..1.0,
                };

                if let Some((color, depth)) = $to_clear {
                    $builder.clear_color_image(ClearColorImageInfo {
                        clear_value: [0., 0., 0., 0.].into(), ..ClearColorImageInfo::image(color)
                    }).unwrap();
                    $builder.clear_depth_stencil_image(ClearDepthStencilImageInfo {
                        clear_value: 1.0.into(),
                        ..ClearDepthStencilImageInfo::image(depth)

                    }).unwrap();
                }

                push_render_pass!(@finish clear_values, $builder, $target $(, $secondary)?);
            };
            (@finish $clear_values:ident, $builder:ident, $target:ident, $secondary:expr) => {
                $builder
                    .begin_render_pass(RenderPassBeginInfo {
                        clear_values: $clear_values.clone(),
                        ..RenderPassBeginInfo::framebuffer($target.clone())
                    }, SubpassContents::SecondaryCommandBuffers)
                    .unwrap();

                $builder.execute_commands($secondary).unwrap();

                $builder.end_render_pass().unwrap();
            };
            (@finish $clear_values:ident, $builder:ident, $target:ident) => {
                $builder
                    .begin_render_pass(RenderPassBeginInfo {
                        clear_values: $clear_values,
                        ..RenderPassBeginInfo::framebuffer($target.clone())
                    }, SubpassContents::Inline).unwrap()
                    .bind_vertex_buffers(0, (vertex_buffer.clone()))
                    .bind_index_buffer(index_buffer.clone());
            };
        }

        for command in self.render_commands.drain(..) {
            match command {
                SubPassRenderCommand::RenderPrimitive { target, offset, start, end, to_clear } => {
                    push_render_pass!(builder, target, viewport, dimensions, to_clear);
                    primitive_renderer(
                        &mut builder,
                        &viewport,
                        &dimensions,
                        offset,
                        start as _,
                        end as _,
                    );
                    builder.end_render_pass().unwrap();
                }
                SubPassRenderCommand::Raw {
                    target,
                    fun,
                    z_index,
                    rect,
                    clipping_rect: _,
                    to_clear,
                } => {
                    push_render_pass!(
                        builder,
                        target,
                        viewport,
                        dimensions,
                        to_clear,
                        fun(
                            &viewport,
                            1.0 - z_index as f32 / 65535.0,
                            rect,
                            Physical::new(dimensions.into()),
                        )
                    );
                }
                SubPassRenderCommand::ResolveOrFinish {
                    resolve,
                    depth,
                    color,
                    to,
                    abs_rect,
                    rect,
                    z_index,
                    to_clear,
                } => {
                    push_render_pass!(
                        builder,
                        to,
                        viewport,
                        dimensions,
                        to_clear,
                        resolve(
                            callback_context,
                            color,
                            depth,
                            self.render_pass.clone(),
                            viewport.clone(),
                            dimensions,
                            abs_rect,
                            rect,
                            1.0 - z_index as f32 / 65535.0,
                        )
                    );
                }
            }
        }

        builder.build().unwrap()
    }
}
