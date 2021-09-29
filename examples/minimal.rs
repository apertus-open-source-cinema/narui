use narui::{
    app,
    column,
    flexible,
    rect_leaf,
    rsx,
    rsx_toplevel,
    widget,
    Color,
    Fragment,
    WidgetContext,
};

#[widget]
pub fn top(context: &mut WidgetContext) -> Fragment {
    let color1 = Color::new(1., 0., 0., 1.);
    let color2 = Color::new(0., 1., 0., 1.);

    rsx! {
        <column>
            <flexible flex=1.0>
                <rect_leaf fill=Some(color1) />
            </flexible>
            <flexible flex=2.0>
                <rect_leaf fill=Some(color2) />
            </flexible>
        </column>
    }
}

fn main() {
    env_logger::init();
    app::render(
        app::WindowBuilder::new().with_title("narui minimal demo"),
        rsx_toplevel! {
            <top />
        },
    );
}
