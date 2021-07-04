use narui::*;
use stretch::style::{AlignItems, Dimension, JustifyContent};
use winit::{platform::unix::WindowBuilderExtUnix, window::WindowBuilder};

#[widget(initial_value = 1, step_size = 1)]
pub fn counter(initial_value: i32, step_size: i32, context: Context) -> Fragment {
    let count = context.listenable(initial_value);

    rsx! {
         <row align_items=AlignItems::Center justify_content=JustifyContent::Center>
            <button on_click=move |context: Context, _state| context.shout(count, context.listen(count) - 1)>
                <text>{" - ".to_string()}</text>
            </button>
            <padding all=Dimension::Points(10.)>
                <text>{format!("{}", context.listen(count))}</text>
            </padding>
            <button on_click=move |context: Context, _state| context.shout(count, context.listen(count) + 1)>
                <text>{" + ".to_string()}</text>
            </button>
         </row>
    }
}


fn main() {
    let window_builder = WindowBuilder::new()
        .with_title("narui counter demo")
        .with_gtk_theme_variant("dark".parse().unwrap());

    render(
        window_builder,
        rsx! {
            <counter />
        },
    );
}
