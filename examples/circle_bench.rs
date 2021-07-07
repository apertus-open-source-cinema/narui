use narui::*;
use stretch::style::{AlignItems, Dimension, JustifyContent};
use winit::{platform::unix::WindowBuilderExtUnix, window::WindowBuilder};


fn main() {
    let window_builder = WindowBuilder::new()
        .with_title("narui counter demo")
        .with_gtk_theme_variant("dark".parse().unwrap());

    render(
        window_builder,
        rsx_toplevel! {
            <row justify_content={JustifyContent::SpaceEvenly} fill_parent=true>
                {(0..50).map(|x| rsx!{
                    <column justify_content={JustifyContent::SpaceEvenly} align_items={AlignItems::Center} fill_parent=true key=&x>
                        {(0..50).map(|y| rsx!{
                            <rounded_rect
                                key=&y
                                fill_color={
                                    let listenable: Listenable<u64> = unsafe { Listenable::uninitialized(Key::default().with(KeyPart::sideband("frame_counter"))) };
                                    let val = context.listen(listenable);
                                    Some(Color::from_components((x as f32 / 50., y as f32 / 50., ((val as f32 / 10.0).sin() + 1.) / 2., 1.)))
                                }
                            >
                                <min_size width={Dimension::Points(10.0)} height={Dimension::Points(10.0)} />
                            </rounded_rect>
                        }).to_fragment(context.clone())}
                    </column>
                }).to_fragment(context.clone())}
            </row>
        },
    );
}
