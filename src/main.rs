use narui::{heart::*, theme, vulkano_render::render, widgets::*};
use narui_derive::{rsx, toplevel_rsx};
use stretch::style::{AlignItems, Dimension, JustifyContent, Style};
use winit::{platform::unix::WindowBuilderExtUnix, window::WindowBuilder};

fn main() {
    let window_builder = WindowBuilder::new()
        .with_title("narui counter demo")
        .with_gtk_theme_variant("dark".parse().unwrap());


    /*render(
        window_builder,
        toplevel_rsx! {
            <row justify_content={JustifyContent::SpaceEvenly} fill_parent=true>
                {(0..80).map(|x| rsx!{
                    <column justify_content={JustifyContent::SpaceEvenly} align_items={AlignItems::Center} fill_parent=true key=x>
                        {(0..50).map(|y| rsx!{
                            <rounded_rect color=Color::from_components((x as f32 / 33., y as f32 / 31., 0., 1.)) key=y>
                                <min_size width={Dimension::Points(10.0)} height={Dimension::Points(10.0)}>
                                    {vec![]}
                                </min_size>
                            </rounded_rect>
                        }).collect::<Vec<_>>()}
                    </column>
                }).collect::<Vec<_>>()}
            </row>
        },
    );*/

    render(
        window_builder,
        toplevel_rsx! {
            <row align_items=AlignItems::Center justify_content=JustifyContent::Center>
                <slider_demo />
            </row>
        },
    );
}
