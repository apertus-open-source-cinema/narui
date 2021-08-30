use narui::*;
use narui_widgets::*;
use winit::{platform::unix::WindowBuilderExtUnix, window::WindowBuilder};


#[widget]
pub fn btn(context: &mut WidgetContext) -> Fragment {
    let clicked = context.listenable(false);
    let color = if context.listen(clicked) { color!(#222222) } else { color!(#ffffff) };

    let callback = move |context: &CallbackContext, is_clicked| {
        context.shout(clicked, is_clicked);
    };

    rsx! {
        <rect fill=Some(color)>
            <input_leaf on_click = callback />
        </rect>
    }
}


fn main() {
    env_logger::init();
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
