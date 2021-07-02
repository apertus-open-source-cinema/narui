use narui::{heart::*, theme, vulkano_render::render, widgets::*};
use narui::macros::{rsx};
use stretch::style::{AlignItems, Dimension, JustifyContent, Style};
use winit::{platform::unix::WindowBuilderExtUnix, window::WindowBuilder};


fn main() {
    let window_builder = WindowBuilder::new()
        .with_title("narui counter demo")
        .with_gtk_theme_variant("dark".parse().unwrap());

/*
    render(
        window_builder,
        rsx! {
            <row justify_content={JustifyContent::SpaceEvenly} fill_parent=true>
                {(0..33).map(|x| rsx!{
                    <column justify_content={JustifyContent::SpaceEvenly} align_items={AlignItems::Center} fill_parent=true key=&x>
                        {(0..31).map(|y| rsx!{
                            <rounded_rect color=Color::from_components((x as f32 / 33., y as f32 / 31., 0., 1.)) key=&y>
                                <min_size width={Dimension::Points(10.0)} height={Dimension::Points(10.0)} />
                            </rounded_rect>
                        }).collect()}
                    </column>
                }).collect()}
            </row>
        },
    );
 */
/*
    render(
        window_builder,
        rsx! {
            <frame_counter />
        },
    );
 */

    render(
        window_builder,
        rsx! {
            <slider_demo />
        },
    );
}
