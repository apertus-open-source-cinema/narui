use narui::{widget, Fragment, FragmentInner, WidgetContext};
use rutter_layout::Transparent;

#[widget]
pub fn fragment(children: Option<Fragment>, context: &mut WidgetContext) -> FragmentInner {
    let children = if let Some(child) = children {
        narui::smallvec![child]
    } else {
        narui::smallvec![]
    };
    FragmentInner::Node { children, layout: Box::new(Transparent), is_clipper: false }
}
