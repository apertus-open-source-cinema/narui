use narui::{api::Widget, hooks::Context, layout::do_layout, render::render, widgets::*};
use narui_derive::rsx;
use stretch::style::{Dimension, JustifyContent};

fn main() -> Result<(), stretch::Error> {
    let __context: Context = Default::default();
    let top_node = rsx! {
        <column justify_content={JustifyContent::SpaceBetween}>
            <column fill_parent=false>
                <rounded_rect width={Dimension::Points(200.0)} height={Dimension::Points(100.0)}/>
                <rounded_rect width={Dimension::Points(200.0)} height={Dimension::Points(100.0)}/>
                <rounded_rect width={Dimension::Points(200.0)} height={Dimension::Points(100.0)}/>
            </column>
            <rounded_rect width={Dimension::Points(200.0)} height={Dimension::Points(100.0)}/>
            <rounded_rect width={Dimension::Points(200.0)} height={Dimension::Points(100.0)}/>
            <rounded_rect width={Dimension::Points(200.0)} height={Dimension::Points(100.0)}/>
        </column>
    };

    render(top_node);

    Ok(())
}
