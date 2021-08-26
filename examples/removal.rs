use narui::*;
use narui_widgets::*;
use winit::{platform::unix::WindowBuilderExtUnix, window::WindowBuilder};

#[widget]
pub fn top(context: &mut WidgetContext) -> Fragment {
    let frame_counter = context.listenable(0);
    context.after_frame(move |context| {
        context.shout(frame_counter, context.spy(frame_counter) + 1);
    });
    let frame_count = (context.listen(frame_counter) / 50) % 50;
    log::trace!("num children: {}", frame_count);

    rsx! {
        <row main_axis_alignment=MainAxisAlignment::SpaceEvenly>
            {(0..frame_count).map(|x| rsx!{
                <sized_box constraint=BoxConstraints::tight_for(rutter_layout::Size::new(10.0, 10.0)) key=x>
                    <fill_rect
                        color={
                            let val = context.listen(frame_counter);
                            Color::from_components((x as f32 / 50., 0.0, ((val as f32 / 10.0).sin() + 1.) / 2., 1.))
                        }
                    />
                </sized_box>
            }).collect()}
        </row>
    }
}

fn main() {
    env_logger::init();
    let window_builder = WindowBuilder::new()
        .with_title("narui removal test")
        .with_gtk_theme_variant("dark".parse().unwrap());

    render(
        window_builder,
        rsx_toplevel! {
            <top />
        },
    );
}
