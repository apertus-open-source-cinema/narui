use narui::*;
use narui_widgets::*;
use std::{
    sync::mpsc::RecvTimeoutError,
    time::{Duration, SystemTime},
};
use winit::{platform::unix::WindowBuilderExtUnix, window::WindowBuilder};

#[derive(Clone)]
enum Message {
    Stop,
    ButtonClick,
}

#[widget(font_size = 100.)]
pub fn stopwatch(font_size: f32, context: &mut WidgetContext) -> Fragment {
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
    let window_builder = WindowBuilder::new()
        .with_title("narui stopwatch demo")
        .with_gtk_theme_variant("dark".parse().unwrap());

    render(
        window_builder,
        rsx_toplevel! {
            <stopwatch />
        },
    );
}
