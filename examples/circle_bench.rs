use narui::{style::*, *};
use winit::{platform::unix::WindowBuilderExtUnix, window::WindowBuilder};

#[widget]
pub fn top(context: Context) -> Fragment {
    let frame_counter = context.listenable(0);
    context.after_frame(move |context| {
        context.shout(frame_counter, context.listen(frame_counter) + 1);
    });

    rsx!{
        <row justify_content={JustifyContent::SpaceEvenly} fill_parent=true>
            {(0..50).map(|x| rsx!{
                <column justify_content={JustifyContent::SpaceEvenly} align_items={AlignItems::Center} fill_parent=true key=&x>
                    {(0..50).map(|y| rsx!{
                        <rect
                            key=&y
                            fill_color={
                                let val = context.listen(frame_counter);
                                Some(Color::from_components((x as f32 / 50., y as f32 / 50., ((val as f32 / 10.0).sin() + 1.) / 2., 1.)))
                            }
                        >
                            <min_size width={Dimension::Points(10.0)} height={Dimension::Points(10.0)} />
                        </rect>
                    }).collect()}
                </column>
            }).collect()}
        </row>
    }
}


fn main() {
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
