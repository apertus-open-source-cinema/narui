use narui::{layout::layout_trait::*, *};

#[derive(Debug, PartialEq)]
struct RevealClipLayout {
    reveal: f32,
}
impl Layout for RevealClipLayout {
    fn layout(&self, constraint: BoxConstraints, children: LayoutableChildren) -> (Size, u32) {
        assert_eq!(children.len(), 1);

        if let Some(child) = children.into_iter().last() {
            let mut size = child.layout(constraint);
            child.set_pos(Offset::zero());
            child.set_z_index_offset(0);
            size.0 = constraint.maximal_bounded();
            size.0.width *= self.reveal;
            size
        } else {
            panic!()
        }
    }
}

#[widget]
pub fn reveal_box(
    children: FragmentChildren,
    reveal: f32,
    context: &mut WidgetContext,
) -> FragmentInner {
    FragmentInner::Node {
        children,
        layout: Box::new(RevealClipLayout { reveal }),
        is_clipper: true,
        subpass: None,
    }
}

#[widget]
pub fn top(context: &mut WidgetContext) -> Fragment {
    let slider_value = context.listenable(0.75);
    rsx! {
        <align>
            <sized constraint=BoxConstraints::tight(500., 500.)>
                <column cross_axis_alignment=CrossAxisAlignment::Start>
                    <flexible>
                        <padding padding=EdgeInsets::all(10.)>
                            <reveal_box reveal=context.listen(slider_value)>
                                <align alignment=Alignment::center()>
                                    <rect fill=Some(Color::new(0., 0.7, 0.7, 1.0)) border_radius=Fraction(1.) do_clipping=true>
                                        <text size=100.>
                                            {"some really long text, that gets clipped..."}
                                        </text>
                                    </rect>
                                </align>
                            </reveal_box>
                        </padding>
                    </flexible>
                    <slider
                        val={context.listen(slider_value)}
                        on_change={move |context: &CallbackContext, new_val| {
                            context.shout(slider_value, new_val)
                        }}
                        min=0.0 max=1.0
                    />
                </column>
            </sized>
        </align>
    }
}

fn main() {
    env_logger::init();
    app::render(
        app::WindowBuilder::new().with_title("narui clipping demo"),
        rsx_toplevel! {
            <top />
        },
    );
}
