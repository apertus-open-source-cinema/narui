use narui::{widget, Fragment, FragmentInner, WidgetContext};
use rutter_layout::Transparent;

#[widget]
pub fn fragment(children: Vec<Fragment>, context: &mut WidgetContext) -> FragmentInner {
    FragmentInner::Node { children, layout: Box::new(Transparent) }
}
