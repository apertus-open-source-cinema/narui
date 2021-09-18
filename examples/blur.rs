use narui::*;

#[widget]
pub fn draggable_rect(initial_pos: Vec2, context: &mut WidgetContext) -> Fragment {
    let clicked = context.listenable(false);
    let drag_start_pos = context.listenable(Vec2::zero());
    let pos = context.listenable(initial_pos);
    let offset = context.listenable(Vec2::zero());
    let abs_pos = context.listen(pos) + context.listen(offset);

    rsx! {
        <positioned pos=AbsolutePosition::from_offset(abs_pos.into())>
            <input on_move = move |context, _, abs_pos| {
                if context.spy(clicked) {
                    context.shout(offset, abs_pos - context.spy(drag_start_pos));
                }
            }
            on_click = move |context, is_clicked, _, abs_pos| {
                context.shout(clicked, is_clicked);
                context.shout(drag_start_pos, abs_pos);
                if !is_clicked {
                    context.shout(pos, context.spy(offset) + context.spy(pos));
                    context.shout(offset, Vec2::zero());
                }
            }>
                <rect stroke=Some((Color::new(0.0, 1.0, 0., 1.0), 10.0)) constraint=BoxConstraints::tight(200.0, 200.0) />
            </input>
        </positioned>
    }
}

#[widget]
pub fn top(context: &mut WidgetContext) -> Fragment {
    rsx! {
        <blur sigma=1.>
            <stack>
                <backdrop_blur sigma=10.>
                    <stack>
                        <positioned><rect fill=Some(Color::new(1.0, 0.0, 0.0, 1.0))/></positioned>
                        <text size=200.>{"test text"}</text>
                    </stack>
                </backdrop_blur>
                <draggable_rect initial_pos=Vec2::zero() />
                <draggable_rect initial_pos=Vec2::new(100.0, 0.0) />
                <draggable_rect initial_pos=Vec2::new(200.0, 0.0) />
                <draggable_rect initial_pos=Vec2::new(300.0, 0.0) />
                <draggable_rect initial_pos=Vec2::new(400.0, 0.0) />
            </stack>
        </blur>
    }
}

fn main() {
    env_logger::init();
    app::render(
        app::WindowBuilder::new().with_title("narui blur demo"),
        rsx_toplevel! {
            <top />
        },
    );
}
