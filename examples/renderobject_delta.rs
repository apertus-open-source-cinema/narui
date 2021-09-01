use narui::*;

#[widget]
pub fn top(context: &mut WidgetContext) -> Fragment {
    let frame_counter = context.listenable(0);
    context.after_frame(move |context| {
        context.shout(frame_counter, context.spy(frame_counter) + 1);
    });
    let border_radius = ((context.listen(frame_counter) as f32 / 50.0).sin() + 1.0) / 4.0;
    let stroke_width = ((context.listen(frame_counter) as f32 / 30.0).sin() + 1.0) * 50.0;
    rsx! {
        <rect_leaf stroke=Some((Color::new(0., 1., 0., 1.), stroke_width)) fill=Some(Color::new(1., 0., 0., 1.)) border_radius=Fraction(border_radius)>
        </rect_leaf>
    }
}

fn main() {
    env_logger::init();
    app::render(
        app::WindowBuilder::new().with_title("narui render object delta test"),
        rsx_toplevel! {
            <top />
        },
    );
}
