use narui::*;
use narui_widgets::*;
use winit::{platform::unix::WindowBuilderExtUnix, window::WindowBuilder};

#[widget]
pub fn slider_demo(context: &mut WidgetContext) -> Fragment {
    let slider_value = context.listenable(24.0);
    rsx! {
        <column>
            <sized_box constraint=BoxConstraints::default().with_tight_height(300.0)>
                <align alignment=Alignment::bottom_center()>
                    <text size=context.listen(slider_value)>
                        {format!("{:.1} px", context.listen(slider_value))}
                    </text>
                </align>
            </sized_box>
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
    let window_builder = WindowBuilder::new()
        .with_title("narui slider demo")
        .with_gtk_theme_variant("dark".parse().unwrap());

    render(
        window_builder,
        rsx_toplevel! {
            <slider_demo />
        },
    );
}
