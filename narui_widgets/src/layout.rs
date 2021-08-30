use narui::{layout::*, re_export::smallvec::smallvec, *};
use narui_macros::widget;

#[widget(
    cross_axis_alignment = CrossAxisAlignment::Center,
    main_axis_alignment = MainAxisAlignment::Center,
    main_axis_size = MainAxisSize::Max
)]
pub fn column(
    children: narui::FragmentChildren,
    cross_axis_alignment: narui::layout::CrossAxisAlignment,
    main_axis_alignment: narui::layout::MainAxisAlignment,
    main_axis_size: narui::layout::MainAxisSize,
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
    children: narui::FragmentChildren,
    cross_axis_alignment: narui::layout::CrossAxisAlignment,
    main_axis_alignment: narui::layout::MainAxisAlignment,
    main_axis_size: narui::layout::MainAxisSize,
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
    children: narui::Fragment,
    flex: f32,
    fit: narui::layout::FlexFit,
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
    children: narui::Fragment,
    padding: narui::layout::EdgeInsets,
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
    children: narui::Fragment,
    alignment: narui::layout::Alignment,
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
    children: Option<narui::Fragment>,
    constraint: narui::layout::BoxConstraints,
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
    children: narui::FragmentChildren,
    fit: narui::layout::StackFit,
    alignment: narui::layout::Alignment,
    is_clipper: bool,
    context: &mut WidgetContext,
) -> FragmentInner {
    FragmentInner::Node { children, layout: Box::new(Stack { fit, alignment }), is_clipper }
}

#[widget(pos = AbsolutePosition::zero())]
pub fn positioned(
    children: narui::Fragment,
    pos: narui::layout::AbsolutePosition,
    context: &mut WidgetContext,
) -> FragmentInner {
    FragmentInner::Node {
        children: smallvec![children],
        layout: Box::new(Positioned::new(pos)),
        is_clipper: false,
    }
}
