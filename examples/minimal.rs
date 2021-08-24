use narui::*;
use narui_macros::rsx_toplevel;
use winit::{platform::unix::WindowBuilderExtUnix, window::WindowBuilder};

#[widget]
pub fn top(context: Context) -> Fragment {
    let color = color!(#222222);

    rsx! {
        <column>
            <rect constraints=BoxConstraints::tight_for(200.0, 200.0) fill_color=Some(color)>
            </rect>
            <rect constraints=BoxConstraints::tight_for(200.0, 200.0) fill_color=Some(color)>
            </rect>
        </column
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
