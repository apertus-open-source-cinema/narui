use narui_derive::{rsx, widget, context};
use narui::{use_state, hooks::Context};


#[widget(size=12.0)]
fn button(text: &str, size: f32, children: Vec<()>) {
    //println!("{:#?}", context!());
    //let a = use_state!(12);
    //println!("{}", size);
}

#[widget]
fn text() {

}

#[widget]
fn test_widget(size: f32) {
    rsx! {
        <button text="lol" size=size>
            <text>
            </text>
        </button>
    }
}

fn main() {
    /*
        rsx! {
            <stacked>
                <rounded_rect/>
                <text size={20} color={"#fff"}>{format!("{:d}", value)}</text>
            </stacked>
        };
    */

    let __context: Context = Default::default();
    rsx! {
        <test_widget size=12.0/>
    }
}
