use crate::{widget, Context, Fragment, FragmentInner};

#[widget]
pub fn fragment(children: Vec<Fragment>, context: Context) -> FragmentInner {
    FragmentInner { children, layout_object: None }
}
