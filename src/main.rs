use narui::{api::Widget, hooks::Context, types::Color, vulkano_render::render, widgets::*};
use narui_derive::{rsx, toplevel_rsx};
use stretch::style::{Dimension, JustifyContent};

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

    render(toplevel_rsx! {
        <column>
            <button>
                <text>{"Lorem Ipsum is simply dummy text of the printing and typesetting industry. Lorem Ipsum has been the industry's standard dummy text ever since the 1500s, when an unknown printer took a galley of type and scrambled it to make a type specimen book. It has survived not only five centuries, but also the leap into electronic typesetting, remaining essentially unchanged. It was popularised in the 1960s with the release of Letraset sheets containing Lorem Ipsum passages, and more recently with desktop publishing software like Aldus PageMaker including versions of Lorem Ipsum.".to_string()}</text>
            </button>
            <text size={100.}>{"Hallo, welt".to_string()}</text>
            <text size={100.}>{"Hallo, welt".to_string()}</text>
            <text size={100.}>{"Hallo, welt".to_string()}</text>
        </column>
    });
}
