use narui::{style::*, *};
use winit::{platform::unix::WindowBuilderExtUnix, window::WindowBuilder};

#[widget(initial_value = 1)]
pub fn counter(initial_value: i32, context: Context) -> Fragment {
    let count = context.listenable(initial_value);
    let value = context.listen(count);

    rsx! {
         <row align_items=AlignItems::Center justify_content=JustifyContent::Center>
            <button on_click=move |context: Context, _state| context.shout(count, context.listen(count) - 1)>
                <text>{" - ".to_string()}</text>
            </button>
            <text style={STYLE.padding(Points(10.))}>{format!("{}", value)}</text>
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
        rsx_toplevel! {
            <counter />
        },
    );
}
