use narui::*;
use narui_macros::rsx_toplevel;
use winit::{platform::unix::WindowBuilderExtUnix, window::WindowBuilder};

#[widget]
pub fn top(context: &mut WidgetContext) -> Fragment {
    let color1 = color!(#ff0000);
    let color2 = color!(#00ff00);

    rsx! {
        <column>
            <flexible flex = 1.0>
                <fill_rect color=color1 />
            </flexible>
            <flexible flex = 2.0>
                <fill_rect color=color2 />
            </flexible>
        </column>
    }
}


fn main() {
    let window_builder = WindowBuilder::new()
        .with_title("narui minimal demo")
        .with_gtk_theme_variant("dark".parse().unwrap());

    render(
        window_builder,
        rsx_toplevel! {
            <top />
        },
    );
}
