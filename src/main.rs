use narui::{heart::*, theme, vulkano_render::render, widgets::*};
use narui_derive::{hook, rsx, toplevel_rsx, widget};
use stretch::style::{AlignItems, Dimension, JustifyContent};
use winit::window::WindowBuilder;

fn main() {
    /*render(toplevel_rsx! {
        <row justify_content={JustifyContent::SpaceEvenly} fill_parent=true>
            {(0..33).map(|x| rsx!{
                <column justify_content={JustifyContent::SpaceEvenly} fill_parent=true>
                    {(0..31).map(|y| rsx!{
                        <rounded_rect width={Dimension::Points(10.0)} height={Dimension::Points(10.0)} color=Color::rgb(x as f32 / 33., y as f32 / 31., 0.)/>
                    }).collect()}
                </column>
            }).collect()}
        </row>
    });*/
    let window_builder = WindowBuilder::new().with_title("narui counter demo");
    render(
        window_builder,
        toplevel_rsx! {
            <counter />
        },
    );
}
