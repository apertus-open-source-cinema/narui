use narui::*;

#[widget]
pub fn slider_demo(context: &mut WidgetContext) -> Fragment {
    let slider_value = context.listenable(24.0);
    rsx! {
        <column>
            <sized constraint=BoxConstraints::default().with_tight_height(300.0)>
                <align alignment=Alignment::bottom_center()>
                    <text size=context.listen(slider_value)>
                        {format!("{:.1} px", context.listen(slider_value))}
                    </text>
                </align>
            </sized>
            <slider
                val={context.listen(slider_value)}
                on_change={move |context: &CallbackContext, new_val| {
                    context.shout(slider_value, new_val)
                }}
                min=12.0 max=300.0
            />
        </column>
    }
}

fn main() {
    env_logger::init();
    app::render(
        app::WindowBuilder::new().with_title("narui slider demo"),
        rsx_toplevel! {
            <slider_demo />
        },
    );
}
