use narui::*;

#[widget(initial_value = 1)]
pub fn counter(initial_value: i32, context: &mut WidgetContext) -> Fragment {
    let count = context.listenable(initial_value);
    let value = context.listen(count);


    rsx! {
         <row>
            <button on_click=move |context: &CallbackContext| context.shout(count, context.spy(count) - 1)>
                <text>{" - "}</text>
            </button>

            <padding>
                <text>{format!("{}", value)}</text>
            </padding>

            <button on_click=move |context: &CallbackContext| context.shout(count, context.spy(count) + 1)>
                <text>{" + "}</text>
            </button>
         </row>
    }
}


fn main() {
    env_logger::init();
    app::render(
        app::WindowBuilder::new().with_title("narui counter demo"),
        rsx_toplevel! {
            <counter />
        },
    );
}
