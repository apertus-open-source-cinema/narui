use narui::{layout::*, re_export::smallvec::smallvec, *};
use narui_macros::widget;

#[widget]
pub fn column(
    children: FragmentChildren,
    #[default(CrossAxisAlignment::Center)] cross_axis_alignment: CrossAxisAlignment,
    #[default(MainAxisAlignment::Center)] main_axis_alignment: MainAxisAlignment,
    #[default(MainAxisSize::Max)] main_axis_size: MainAxisSize,
    context: &mut WidgetContext,
) -> FragmentInner {
    FragmentInner::Node {
        children,
        layout: Box::new(Column { cross_axis_alignment, main_axis_alignment, main_axis_size }),
        is_clipper: false,
        subpass: None,
    }
}

#[widget]
pub fn row(
    children: FragmentChildren,
    #[default(CrossAxisAlignment::Center)] cross_axis_alignment: CrossAxisAlignment,
    #[default(MainAxisAlignment::Center)] main_axis_alignment: MainAxisAlignment,
    #[default(MainAxisSize::Max)] main_axis_size: MainAxisSize,
    context: &mut WidgetContext,
) -> FragmentInner {
    FragmentInner::Node {
        children,
        layout: Box::new(Row { cross_axis_alignment, main_axis_alignment, main_axis_size }),
        is_clipper: false,
        subpass: None,
    }
}

#[widget]
pub fn flexible(
    children: Fragment,
    #[default(1.0)] flex: f32,
    #[default(FlexFit::Loose)] fit: FlexFit,
    context: &mut WidgetContext,
) -> FragmentInner {
    FragmentInner::Node {
        children: smallvec![children],
        layout: Box::new(Flexible { flex: Flex { flex, fit } }),
        is_clipper: false,
        subpass: None,
    }
}

#[widget]
pub fn padding(
    children: Fragment,
    #[default(EdgeInsets::all(10.0))] padding: EdgeInsets,
    context: &mut WidgetContext,
) -> FragmentInner {
    FragmentInner::Node {
        children: smallvec![children],
        layout: Box::new(Padding::new(padding)),
        is_clipper: false,
        subpass: None,
    }
}

#[widget]
pub fn align(
    children: Fragment,
    #[default(Alignment::center())] alignment: Alignment,
    #[default] factor_width: Option<f32>,
    #[default] factor_height: Option<f32>,
    context: &mut WidgetContext,
) -> FragmentInner {
    FragmentInner::Node {
        children: smallvec![children],
        layout: Box::new(Align::fractional(alignment, factor_width, factor_height)),
        is_clipper: false,
        subpass: None,
    }
}

#[widget]
pub fn sized(
    children: Option<Fragment>,
    constraint: BoxConstraints,
    context: &mut WidgetContext,
) -> FragmentInner {
    FragmentInner::Node {
        children: children.into_iter().collect(),
        layout: Box::new(SizedBox::constrained(constraint)),
        is_clipper: false,
        subpass: None,
    }
}

#[widget]
pub fn stack(
    children: FragmentChildren,
    #[default(StackFit::Loose)] fit: StackFit,
    #[default(Alignment::center())] alignment: Alignment,
    #[default] is_clipper: bool,
    context: &mut WidgetContext,
) -> FragmentInner {
    FragmentInner::Node {
        children,
        layout: Box::new(Stack { fit, alignment }),
        is_clipper,
        subpass: None,
    }
}

#[widget]
pub fn positioned(
    children: Fragment,
    #[default] pos: AbsolutePosition,
    #[default(false)] z_top: bool,
    context: &mut WidgetContext,
) -> FragmentInner {
    FragmentInner::Node {
        children: smallvec![children],
        layout: Box::new(if z_top { Positioned::z_top(pos) } else { Positioned::new(pos) }),
        is_clipper: false,
        subpass: None,
    }
}

#[widget]
pub fn aspect_ratio(
    children: Option<Fragment>,
    aspect_ratio: f32,
    context: &mut WidgetContext,
) -> FragmentInner {
    FragmentInner::Node {
        children: children.into_iter().collect(),
        layout: Box::new(AspectRatioBox::new(AspectRatio { ratio: aspect_ratio })),
        is_clipper: false,
        subpass: None,
    }
}
