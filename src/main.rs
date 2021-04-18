use narui::{api::Widget, hooks::Context, layout::do_layout, vulkano_render::render, widgets::*};
use narui::types::Color;
use narui_derive::{toplevel_rsx, rsx};
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
