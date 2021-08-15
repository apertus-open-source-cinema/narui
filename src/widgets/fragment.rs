use crate::{Fragment, Context, widget};

#[widget]
pub fn fragment(children: Fragment, context: Context) -> Fragment {
    dbg!("re_render fragment {}", context.widget_local.key);
    dbg!(&children);
    Fragment {
        key: context.widget_local.key,
        children: children.into(),
        layout_object: None,
    }
}