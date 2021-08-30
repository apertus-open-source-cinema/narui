use narui::{layout::Transparent, *};
use narui_macros::widget;

#[widget]
pub fn fragment(children: Option<Fragment>, context: &mut WidgetContext) -> FragmentInner {
    FragmentInner::Node {
        children: children.into_iter().collect(),
        layout: Box::new(Transparent),
        is_clipper: false,
    }
}
