use narui::*;
use std::{
    sync::mpsc::RecvTimeoutError,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use stretch::style::{AlignItems, Dimension, JustifyContent, Style};
use winit::{platform::unix::WindowBuilderExtUnix, window::WindowBuilder};

#[derive(Clone)]
enum Message {
    Stop,
}

#[widget(font_size = 100.)]
pub fn clock(font_size: f32, context: Context) -> Fragment {
    let time = context.listenable("".to_string());
    context.thread(
        move |context, rx| loop {
            let now = SystemTime::now();
            let time_string = format!("{}", now.duration_since(UNIX_EPOCH).unwrap().as_secs());
            context.shout(time, time_string);
            match rx.recv_timeout(Duration::from_secs(1)) {
                Ok(Message::Stop) => return,
                Err(RecvTimeoutError::Timeout) => {}
                _ => panic!(),
            }
        },
        Message::Stop,
        font_size, /* we use this to test the collection of old threads by provoking the
                    * creation of new ones */
    );

    rsx! {
         <text size=font_size>{context.listen(time)}</text>
    }
}

#[widget]
pub fn slider_clock(context: Context) -> Fragment {
    let slider_value = context.listenable(24.0);

    rsx! {
        <column fill_parent=true align_items=AlignItems::Center justify_content=JustifyContent::Center>
            <column fill_parent=false align_items=AlignItems::Center>
                <min_size height=Dimension::Points(300.0) style={Style { align_items: AlignItems::FlexEnd, ..Default::default() }}>
                    <clock font_size=context.listen(slider_value) />
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
        .with_title("narui clock demo")
        .with_gtk_theme_variant("dark".parse().unwrap());

    render(
        window_builder,
        rsx_toplevel! {
            <slider_clock />
        },
    );
}
