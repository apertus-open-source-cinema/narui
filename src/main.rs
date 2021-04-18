use narui::{api::Widget, hooks::Context, layout::do_layout, vulkano_render::render, widgets::*};
use narui_derive::rsx;
use stretch::style::{Dimension, JustifyContent};

fn main() -> Result<(), stretch::Error> {
    let __context: Context = Default::default();


    let top_node = rsx! {
        <row justify_content={JustifyContent::SpaceEvenly} fill_parent=true>
            {(0..33).map(|i| rsx!{
                <column justify_content={JustifyContent::SpaceEvenly} fill_parent=true>
                    {(0..31).map(|i| rsx!{
                        <rounded_rect width={Dimension::Points(10.0)} height={Dimension::Points(10.0)}/>
                    }).collect()}
                </column>
            }).collect()}
        </row>
    };

    render(top_node);

    Ok(())
}
