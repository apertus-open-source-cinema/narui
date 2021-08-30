use narui::*;
use narui_widgets::*;
use rutter_layout::{Layout, LayoutableChildren, Size};
use winit::{platform::unix::WindowBuilderExtUnix, window::WindowBuilder};


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
    }
}

#[widget]
pub fn slider_demo(context: &mut WidgetContext) -> Fragment {
    let slider_value = context.listenable(0.75);
    rsx! {
        <align>
            <sized_box constraint=BoxConstraints::tight(400., 400.)>
                <column cross_axis_alignment=CrossAxisAlignment::Start>
                    <flexible>
                        <padding padding=EdgeInsets::all(10.)>
                            <reveal_box reveal=context.listen(slider_value)>
                                <stack>
                                    <rect fill=Some(color!(#007777)) border_radius=Paxel(15.)/>
                                    <text size=150.>
                                        {"some really long text, that gets clipped..."}
                                    </text>
                                </stack>
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
            </sized_box>
        </align>
    }
}

fn main() {
    env_logger::init();
    let window_builder = WindowBuilder::new()
        .with_title("narui clipping demo")
        .with_gtk_theme_variant("dark".parse().unwrap());

    render(
        window_builder,
        rsx_toplevel! {
            <slider_demo />
        },
    );
}
