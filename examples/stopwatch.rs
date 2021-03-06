use narui::*;
use std::{
    sync::mpsc::RecvTimeoutError,
    time::{Duration, SystemTime},
};

#[derive(Clone)]
enum Message {
    Stop,
    ButtonClick,
}

#[widget]
pub fn stopwatch(#[default(100.)] font_size: f32, context: &mut WidgetContext) -> Fragment {
    let time = context.listenable(0.0);
    let button_text = context.listenable("start");

    let thread_handle = context.thread(
        move |context, rx| loop {
            match rx.recv().unwrap() {
                Message::Stop => return,
                Message::ButtonClick => {}
            }
            context.shout(button_text, "stop");
            let start = SystemTime::now();
            loop {
                let now = SystemTime::now();
                context.shout(time, now.duration_since(start).unwrap().as_secs_f32());
                match rx.recv_timeout(Duration::from_secs_f32(1. / 100.)) {
                    Ok(Message::Stop) => return,
                    Ok(Message::ButtonClick) => break,
                    Err(RecvTimeoutError::Timeout) => {}
                    _ => panic!(),
                }
            }
            context.shout(button_text, "reset");
            match rx.recv().unwrap() {
                Message::Stop => return,
                Message::ButtonClick => {}
            }
            context.shout(time, 0.0);
        },
        Message::Stop,
        (),
    );

    rsx! {
        <column>
            <text size=font_size>{format!("{:.2}", context.listen(time))}</text>
            <button on_click=move |_context: &CallbackContext| { thread_handle.read().send(Message::ButtonClick).unwrap(); }>
                <text>{context.listen(button_text)}</text>
            </button>
        </column>
    }
}


fn main() {
    env_logger::init();
    app::render(
        app::WindowBuilder::new().with_title("narui stopwatch demo"),
        rsx_toplevel! {
            <stopwatch />
        },
    );
}
