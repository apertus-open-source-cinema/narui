use narui::{api::Widget, hooks::Context, types::Color, vulkano_render::render, widgets::*};
use narui_derive::{rsx, toplevel_rsx};
use stretch::style::{Dimension, JustifyContent};

fn main() {
    render(toplevel_rsx! {
        <row justify_content={JustifyContent::SpaceEvenly} fill_parent=true>
            {(0..33).map(|x| rsx!{
                <column justify_content={JustifyContent::SpaceEvenly} fill_parent=true>
                    {(0..31).map(|y| rsx!{
                        <rounded_rect width={Dimension::Points(10.0)} height={Dimension::Points(10.0)} color=Color::rgb(x as f32 / 33., y as f32 / 31., 0.)/>
                    }).collect()}
                </column>
            }).collect()}
        </row>
    });
}
