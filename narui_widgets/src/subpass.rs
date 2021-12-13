use narui_core::{
    layout::Transparent,
    re_export::smallvec::smallvec,
    CallbackContext,
    ContextEffect,
    ContextMeasure,
    Fragment,
    Rect,
    SubPassRenderFunction,
    SubPassSetup,
    Vec2,
    WidgetContext,
};
use narui_macros::{rsx, widget};
use std::sync::Arc;
use vulkano::{
    command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage},
    descriptor_set::PersistentDescriptorSet,
    device::{DeviceOwned, Queue},
    pipeline::{
        graphics::{
            color_blend::{
                AttachmentBlend,
                BlendFactor,
                BlendOp,
                ColorBlendAttachmentState,
                ColorBlendState,
                ColorComponents,
            },
            depth_stencil::{CompareOp, DepthState, DepthStencilState},
            input_assembly::{InputAssemblyState, PrimitiveTopology},
            viewport::ViewportState,
        },
        GraphicsPipeline,
        Pipeline,
        PipelineBindPoint,
        StateMode,
    },
    render_pass::Subpass,
    sampler::{Filter, MipmapMode, Sampler, SamplerAddressMode},
};

mod vertex_shader {
    vulkano_shaders::shader! {
        ty: "vertex",
        src: "
            #version 450
            #extension GL_EXT_debug_printf: enable
            layout(push_constant) uniform PushConstantData {
                uint width;
                uint height;
                float z_index;
                vec2 origin;
                vec2 size;
                ivec2 direction;
                vec3 coeffs;
                vec2 window_min;
                vec2 window_max;
            } params;

            layout(location = 0) out vec2 origin;
            layout(location = 1) out ivec2 direction;
            layout(location = 2) out vec3 coeffs;
            layout(location = 3) out vec2 window_min;
            layout(location = 4) out vec2 window_max;

            void main() {
                int idx = gl_VertexIndex;
                int top = idx & 1;
                int left = (idx & 2) / 2;

                origin = params.origin;
                coeffs = params.coeffs;
                direction = params.direction;
                window_min = params.window_min;
                window_max = params.window_max;

                vec2 pos = params.origin + vec2(top, left) * params.size;
                pos = (pos / (vec2(params.width, params.height) / 2.) - vec2(1.));
                gl_Position = vec4(pos, params.z_index, 1.0);
            }
        "
    }
}
mod fragment_shader {
    vulkano_shaders::shader! {
        ty: "fragment",
        src: "
            #version 450
            #extension GL_EXT_debug_printf: enable
            layout(location = 0) out vec4 f_color;
            layout(set = 0, binding = 0) uniform sampler2DMS depth;
            layout(set = 0, binding = 1) uniform sampler2D color;

            layout(location = 0) flat in vec2 origin;
            layout(location = 1) flat in ivec2 direction;
            layout(location = 2) flat in vec3 coeffs;
            layout(location = 3) flat in vec2 window_min;
            layout(location = 4) flat in vec2 window_max;

            void main() {
                ivec2 pos = ivec2(gl_FragCoord.xy - origin);

                if (pos.x < window_min.x || pos.y < window_min.y
                 || pos.x > window_max.x || pos.y > window_max.y) {
                    f_color = texelFetch(color, pos, 0);
                } else {
                    float sigma = (1. / coeffs.x) / sqrt(2. * 3.14159265358979361);
                    float sampleCount = ceil(1.5 * sigma);

                    vec3 g = coeffs;
                    ivec2 size = textureSize(color, 0) - 1;
                    vec4 color_sum = texelFetch(color, pos, 0) * g.x;
                    float coeff_norm = g.x;
                    for (int i = 1; i < sampleCount; i++) {
                        g.xy *= g.yz;
                        color_sum += texelFetch(color, min(pos + i * direction, size), 0) * g.x;
                        color_sum += texelFetch(color, max(pos - i * direction, ivec2(0)), 0) * g.x;
                        coeff_norm += 2.0 * g.x;
                    }
                    vec4 color = color_sum / coeff_norm;
                    f_color = vec4(color.rgb * color.w, color.w);
                }
            }
        "
    }
}

#[widget]
pub fn raw_blur(
    children: Fragment,
    window: Option<Fragment>,
    sigma: f32,
    in_x: bool,
    backdrop_after: Option<usize>,
    context: &mut WidgetContext,
) -> FragmentInner {
    let pipeline_and_sampler = context.effect(
        |context| {
            let render_pass = context.vulkan_context.render_pass.clone();
            let vs = vertex_shader::load(render_pass.device().clone()).unwrap();
            let fs = fragment_shader::load(render_pass.device().clone()).unwrap();
            let pipeline = GraphicsPipeline::start()
                .vertex_shader(vs.entry_point("main").unwrap(), ())
                .input_assembly_state(
                    InputAssemblyState::new().topology(PrimitiveTopology::TriangleList),
                )
                .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
                .fragment_shader(fs.entry_point("main").unwrap(), ())
                .color_blend_state(ColorBlendState {
                    logic_op: None,
                    attachments: vec![ColorBlendAttachmentState {
                        blend: Some(AttachmentBlend {
                            color_op: BlendOp::Add,
                            color_source: BlendFactor::SrcAlpha,
                            color_destination: BlendFactor::OneMinusSrcAlpha,
                            alpha_op: BlendOp::Max,
                            alpha_source: BlendFactor::One,
                            alpha_destination: BlendFactor::One,
                        }),
                        color_write_mask: ColorComponents::all(),
                        color_write_enable: StateMode::Fixed(true),
                    }],
                    blend_constants: StateMode::Fixed([1.0, 1.0, 1.0, 1.0]),
                })
                .depth_stencil_state(DepthStencilState {
                    depth: Some(DepthState {
                        compare_op: StateMode::Fixed(CompareOp::LessOrEqual),
                        ..Default::default()
                    }),
                    ..DepthStencilState::simple_depth_test()
                })
                .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
                .build(render_pass.device().clone())
                .unwrap();

            let sampler = Sampler::new(
                render_pass.device().clone(),
                Filter::Nearest,
                Filter::Nearest,
                MipmapMode::Nearest,
                SamplerAddressMode::ClampToEdge,
                SamplerAddressMode::ClampToEdge,
                SamplerAddressMode::ClampToEdge,
                0.0,
                1.0,
                0.0,
                0.0,
            )
            .unwrap();

            (pipeline, sampler)
        },
        (),
    );
    let pipeline_and_sampler = pipeline_and_sampler.read();
    let pipeline = pipeline_and_sampler.0.clone();
    let sampler = pipeline_and_sampler.1.clone();
    let queue = context
        .vulkan_context
        .queues
        .iter()
        .find(|&q| q.family().supports_graphics())
        .unwrap()
        .clone();

    fn generate_resolve(
        in_x: bool,
        sigma: f32,
        pipeline: Arc<GraphicsPipeline>,
        sampler: Arc<Sampler>,
        queue: Arc<Queue>,
        window_function: impl Fn(&CallbackContext, Rect, Rect) -> Rect + 'static,
    ) -> SubPassRenderFunction {
        std::rc::Rc::new(
            move |context,
                  color,
                  depth,
                  render_pass,
                  viewport,
                  dimensions,
                  abs_pos,
                  rect,
                  z_index| {
                let window = window_function(context, abs_pos, rect);

                let mut set_builder = PersistentDescriptorSet::start(
                    pipeline.layout().descriptor_set_layouts()[0].clone(),
                );
                set_builder
                    .add_sampled_image(depth, sampler.clone())
                    .unwrap()
                    .add_sampled_image(color, sampler.clone())
                    .unwrap();
                let descriptor_set = set_builder.build().unwrap();

                let coeff_a = 1.0 / ((2.0f32 * 3.151_592_3).sqrt() * sigma);
                let coeff_b = (-0.5 / (sigma * sigma)).exp();
                let coeff_c = coeff_b * coeff_b;

                let push_constants = vertex_shader::ty::PushConstantData {
                    width: dimensions[0],
                    height: dimensions[1],
                    z_index,
                    origin: rect.pos.into(),
                    size: rect.size.into(),
                    coeffs: [coeff_a, coeff_b, coeff_c],
                    direction: if in_x { [1, 0] } else { [0, 1] },
                    window_min: window.top_left_corner().into(),
                    window_max: window.bottom_right_corner().into(),
                    _dummy0: Default::default(),
                    _dummy1: Default::default(),
                    _dummy2: Default::default(),
                };

                let mut builder = AutoCommandBufferBuilder::secondary_graphics(
                    render_pass.device().clone(),
                    queue.family(),
                    CommandBufferUsage::MultipleSubmit,
                    pipeline.subpass().clone(),
                )
                .unwrap();
                builder
                    .bind_descriptor_sets(
                        PipelineBindPoint::Graphics,
                        pipeline.layout().clone(),
                        0,
                        descriptor_set,
                    )
                    .bind_pipeline_graphics(pipeline.clone())
                    .push_constants(pipeline.layout().clone(), 0, push_constants)
                    .set_viewport(0, std::iter::once(viewport))
                    .draw(4, 1, 0, 0)
                    .unwrap();

                builder.build().unwrap()
            },
        )
    }

    let resolve = generate_resolve(
        in_x,
        sigma,
        pipeline.clone(),
        sampler.clone(),
        queue.clone(),
        move |context, abs_pos, rect| match window {
            Some(frag) => {
                let window_rect = context.measure(frag).unwrap();
                window_rect.minus_position(abs_pos.pos)
            }
            None => Rect::from_corners(Vec2::zero(), rect.size),
        },
    );

    let finish =
        generate_resolve(in_x, sigma, pipeline, sampler, queue, move |_, _, _| Rect::zero());

    FragmentInner::Node {
        children: smallvec![children],
        layout: Box::new(Transparent),
        is_clipper: false,
        subpass: Some(SubPassSetup {
            resolve,
            finish: backdrop_after.map(|after| (finish, Some(after))),
        }),
    }
}

#[widget]
pub fn blur(
    children: Fragment,
    #[default] window: Option<Fragment>,
    sigma: f32,
    context: &mut WidgetContext,
) -> Fragment {
    rsx! {
        <raw_blur sigma=sigma in_x=true window=window backdrop_after=None>
            <raw_blur sigma=sigma in_x=false window=window backdrop_after=None>
                {children}
            </raw_blur>
        </raw_blur>
    }
}

#[widget]
pub fn backdrop_blur(children: Fragment, sigma: f32, context: &mut WidgetContext) -> Fragment {
    rsx! {
        <raw_blur sigma=sigma in_x=true window=None backdrop_after=None>
            <raw_blur sigma=sigma in_x=false window=None backdrop_after=Some(1)>
                {children}
            </raw_blur>
        </raw_blur>
    }
}
