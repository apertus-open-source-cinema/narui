use narui::*;

#[widget]
pub fn btn(context: &mut WidgetContext) -> Fragment {
    let clicked = context.listenable(false);
    let color = if context.listen(clicked) {
        Color::new(1., 0., 0., 1.)
    } else {
        Color::new(0., 1., 0., 1.)
    };

    let callback = move |context: &CallbackContext, is_clicked| {
        context.shout(clicked, is_clicked);
    };

    rsx! {
        <input on_click=callback>
            <rect fill=Some(color) constraint=BoxConstraints::fill()/>
        </input>
    }
}


fn main() {
    env_logger::init();
    app::render(
        app::WindowBuilder::new().with_title("narui minimal delta demo"),
        rsx_toplevel! {
            <btn />
        },
    );
}
