use narui::*;

#[widget]
pub fn top(context: &mut WidgetContext) -> Fragment {
    let frame_counter = context.listenable(0);
    context.after_frame(move |context| {
        context.shout(frame_counter, context.spy(frame_counter) + 1);
    });

    rsx! {
        <row main_axis_alignment=MainAxisAlignment::SpaceEvenly>
            {(0..300).map(|x| rsx!{
                <column main_axis_alignment=MainAxisAlignment::SpaceEvenly key=x>
                    {(0..300).map(|y| rsx! {
                        <sized constraint=BoxConstraints::tight(10.0, 10.0) key=y><rect_leaf fill=Some({
                                let val = context.listen(frame_counter);
                                Color::from_components((x as f32 / 50., y as f32 / 50., ((val as f32 / 10.0).sin() + 1.) / 2., 1.))
                        }) /></sized>
                    }).collect()}
                </column>
            }).collect()}
        </row>
    }
}

fn main() {
    env_logger::init();
    app::render(
        app::WindowBuilder::new().with_title("narui circle benchmark"),
        rsx_toplevel! {
            <top />
        },
    );
}
