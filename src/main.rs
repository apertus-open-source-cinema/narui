use narui_derive::{rsx, widget, hook};
use narui::{hooks::Context};
use narui::hooks::state;


#[widget]
fn text(size: f32, children: String) {
    println!("{:#?}", children);
}

#[widget]
fn rounded_rect() {

}

#[widget]
fn stacked(children: Vec<()>) {

}


#[widget(size=12.0, on_click=(|| {}))]
fn button(size: f32, mut on_click: impl FnMut() -> (), children: String) {
    on_click();

    assert_eq!(children.len(), 1);
    rsx! {
        <stacked>
            <rounded_rect/>
            <text size={size}>{children}</text>
        </stacked>
    };
}

#[widget]
fn counter() {
    let count = hook!(state(0));

    rsx! {
        <button on_click={|| count.set(*count + 1)}>
            {format!("{}", *count)}
        </button>
    }
}

fn main() {
    let __context: Context = Default::default();
    for i in 0..10 {
        rsx! { <counter /> };
    }
}
