use narui::{layout::*, re_export::smallvec::smallvec, *};
use narui_macros::widget;

#[widget(
    cross_axis_alignment = CrossAxisAlignment::Center,
    main_axis_alignment = MainAxisAlignment::Center,
    main_axis_size = MainAxisSize::Max
)]
pub fn column(
    children: FragmentChildren,
    cross_axis_alignment: CrossAxisAlignment,
    main_axis_alignment: MainAxisAlignment,
    main_axis_size: MainAxisSize,
    context: &mut WidgetContext,
) -> FragmentInner {
    FragmentInner::Node {
        children,
        layout: Box::new(Column { cross_axis_alignment, main_axis_alignment, main_axis_size }),
        is_clipper: false,
    }
}

#[widget(
    cross_axis_alignment = CrossAxisAlignment::Center,
    main_axis_alignment = MainAxisAlignment::Center,
    main_axis_size = MainAxisSize::Max
)]
pub fn row(
    children: FragmentChildren,
    cross_axis_alignment: CrossAxisAlignment,
    main_axis_alignment: MainAxisAlignment,
    main_axis_size: MainAxisSize,
    context: &mut WidgetContext,
) -> FragmentInner {
    FragmentInner::Node {
        children,
        layout: Box::new(Row { cross_axis_alignment, main_axis_alignment, main_axis_size }),
        is_clipper: false,
    }
}

#[widget(flex = 1.0, fit = FlexFit::Loose)]
pub fn flexible(
    children: Fragment,
    flex: f32,
    fit: FlexFit,
    context: &mut WidgetContext,
) -> FragmentInner {
    FragmentInner::Node {
        children: smallvec![children],
        layout: Box::new(Flexible { flex: Flex { flex, fit } }),
        is_clipper: false,
    }
}

#[widget(padding = EdgeInsets::all(10.0))]
pub fn padding(
    children: Fragment,
    padding: EdgeInsets,
    context: &mut WidgetContext,
) -> FragmentInner {
    FragmentInner::Node {
        children: smallvec![children],
        layout: Box::new(Padding::new(padding)),
        is_clipper: false,
    }
}

#[widget(
    alignment = Alignment::center(),
    factor_width = None,
    factor_height = None
)]
pub fn align(
    children: Fragment,
    alignment: Alignment,
    factor_width: Option<f32>,
    factor_height: Option<f32>,
    context: &mut WidgetContext,
) -> FragmentInner {
    FragmentInner::Node {
        children: smallvec![children],
        layout: Box::new(Align::fractional(alignment, factor_width, factor_height)),
        is_clipper: false,
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
    }
}

#[widget(fit = StackFit::Loose, alignment = Alignment::center(), is_clipper = false)]
pub fn stack(
    children: FragmentChildren,
    fit: StackFit,
    alignment: Alignment,
    is_clipper: bool,
    context: &mut WidgetContext,
) -> FragmentInner {
    FragmentInner::Node { children, layout: Box::new(Stack { fit, alignment }), is_clipper }
}

#[widget(pos = AbsolutePosition::zero())]
pub fn positioned(
    children: Fragment,
    pos: AbsolutePosition,
    context: &mut WidgetContext,
) -> FragmentInner {
    FragmentInner::Node {
        children: smallvec![children],
        layout: Box::new(Positioned::new(pos)),
        is_clipper: false,
    }
}
