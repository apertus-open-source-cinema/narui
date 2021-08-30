use narui::*;
use narui_widgets::*;
use winit::{platform::unix::WindowBuilderExtUnix, window::WindowBuilder};

#[widget]
pub fn top(context: &mut WidgetContext) -> Fragment {
    let frame_counter = context.listenable(0);
    context.after_frame(move |context| {
        context.shout(frame_counter, context.spy(frame_counter) + 1);
    });
    let _frame_count = context.listen(frame_counter);

    rsx! {
        <row main_axis_alignment=MainAxisAlignment::SpaceEvenly>
            {(0..100).map(|x| rsx!{
                <column main_axis_alignment=MainAxisAlignment::SpaceEvenly key=x>
                    {(0..100).map(|y| rsx! {
                        <sized_box constraint=BoxConstraints::tight(10.0, 10.0) key=y>
                            <rect_leaf
                                fill=Some({
                                    let val = context.listen(frame_counter);
                                    Color::from_components((x as f32 / 50., y as f32 / 50., ((val as f32 / 10.0).sin() + 1.) / 2., 1.))
                                })
                            />
                        </sized_box>
                    }).collect()}
                </column>
            }).collect()}
        </row>
    }
}

fn main() {
    env_logger::init();
    let window_builder = WindowBuilder::new()
        .with_title("narui circle benchmark")
        .with_gtk_theme_variant("dark".parse().unwrap());

    render(
        window_builder,
        rsx_toplevel! {
            <top />
        },
    );
}
