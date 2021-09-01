use narui::*;

#[widget]
pub fn top(context: &mut WidgetContext) -> Fragment {
    let frame_counter = context.listenable(0);
    context.after_frame(move |context| {
        context.shout(frame_counter, context.spy(frame_counter) + 1);
    });
    let a = (context.listen(frame_counter)) % 200;
    let frame_count = if a > 100 { 200 - a } else { a } + 50;
    log::trace!("num children: {}", frame_count);

    rsx! {
        <row main_axis_alignment=MainAxisAlignment::SpaceEvenly>
            {(0..frame_count).map(|x| rsx!{
                <column main_axis_alignment=MainAxisAlignment::SpaceEvenly key=x>
                    {(0..frame_count).map(|y| rsx! {
                        <sized constraint=BoxConstraints::tight(10.0, 10.0) key=y>
                            <rect_leaf
                                key=y
                                fill=Some({
                                    let val = context.listen(frame_counter);
                                    Color::from_components((x as f32 / 50., y as f32 / 50., ((val as f32 / 10.0).sin() + 1.) / 2., 1.))
                                })
                            />
                        </sized>
                    }).collect()}
                </column>
            }).collect()}
        </row>
    }
}

fn main() {
    env_logger::init();
    app::render(
        app::WindowBuilder::new().with_title("narui minimal test"),
        rsx_toplevel! {
            <top />
        },
    );
}
