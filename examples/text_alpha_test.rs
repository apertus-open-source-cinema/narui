use narui::*;

#[widget]
pub fn top(context: &mut WidgetContext) -> Fragment {
    let c = Color::new(1.0, 0., 0., 1.0);
    let b = Color::new(0.0, 1.0, 0., 0.5);
    rsx! {
        <stack>
            <rect fill=Some(c)></rect>
            <text size=300.0>{"test text"}</text>
            <rect fill=Some(b) constraint=BoxConstraints::tight(400.0, 400.0)></rect>
        </stack>
    }
}

fn main() {
    env_logger::init();
    app::render(
        app::WindowBuilder::new().with_title("narui text alpha demo"),
        rsx_toplevel! {
            <top />
        },
    );
}
