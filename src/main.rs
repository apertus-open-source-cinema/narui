use lyon::{
    math::{rect, Point},
    path::{builder::*, Winding},
    tessellation::{path, path::path::Builder},
};
use narui::{
    api::{RenderObject, TreeChildren, Widget},
    hooks::{state, Context},
    layout::do_layout,
    widgets::*,
};
use narui_derive::{hook, rsx, widget};
use stretch::{
    geometry::{Rect, Size},
    style::{
        AlignContent,
        AlignItems,
        Dimension,
        FlexDirection,
        FlexWrap,
        JustifyContent,
        PositionType,
        Style,
    },
};


fn main() {
    let __context: Context = Default::default();
    let top_node = rsx! {
        <column justify_content={JustifyContent::SpaceBetween}>
            <column fill_parent=false>
                <rounded_rect width={Dimension::Points(10.0)} height={Dimension::Points(10.0)}/>
                <rounded_rect width={Dimension::Points(10.0)} height={Dimension::Points(10.0)}/>
                <rounded_rect width={Dimension::Points(10.0)} height={Dimension::Points(10.0)}/>
            </column>
            <rounded_rect width={Dimension::Points(10.0)} height={Dimension::Points(10.0)}/>
            <rounded_rect width={Dimension::Points(10.0)} height={Dimension::Points(10.0)}/>
            <rounded_rect width={Dimension::Points(10.0)} height={Dimension::Points(10.0)}/>
        </column>
    };

    dbg!(do_layout(top_node));
}
