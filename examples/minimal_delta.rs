use narui::*;
use narui_macros::rsx_toplevel;
use stretch::style::{AlignItems, Dimension, JustifyContent};
use winit::{platform::unix::WindowBuilderExtUnix, window::WindowBuilder};

#[widget]
pub fn btn(context: Context) -> Fragment {
    let clicked = context.listenable(false);
    let color = if context.listen(clicked) { color!(#222222) } else { color!(#ffffff) };

    let callback = move |context: Context, is_clicked| {
        context.shout(clicked, is_clicked);
    };

    rsx! {
        <rounded_rect fill_color=Some(color)>
            <input on_click=callback.clone()>
                <min_size width={Dimension::Points(100.0)} height={Dimension::Points(100.0)} />
            </input>
        </rounded_rect>
    }
}


fn main() {
    let window_builder = WindowBuilder::new()
        .with_title("narui minimal delta demo")
        .with_gtk_theme_variant("dark".parse().unwrap());

    render(
        window_builder,
        rsx_toplevel! {
            <btn />
        },
    );
}
