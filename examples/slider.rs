use narui::*;
use narui_macros::rsx_toplevel;
use stretch::style::{AlignItems, Dimension, JustifyContent, Style};
use winit::{platform::unix::WindowBuilderExtUnix, window::WindowBuilder};


#[widget]
pub fn slider_demo(context: Context) -> Fragment {
    let slider_value = context.listenable(24.0);

    rsx! {
        <column fill_parent=true align_items=AlignItems::Center justify_content=JustifyContent::Center>
            <column fill_parent=false align_items=AlignItems::Center>
                <min_size height=Dimension::Points(300.0) style={Style { align_items: AlignItems::FlexEnd, ..Default::default() }}>
                    <text size=context.listen(slider_value)>
                        {format!("{:.1} px", context.listen(slider_value))}
                    </text>
                </min_size>
                <slider
                    val={context.listen(slider_value)}
                    on_change={move |context: Context, new_val| {
                        context.shout(slider_value, new_val)
                    }}
                    min=12.0 max=300.0
                />
            </column>
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
