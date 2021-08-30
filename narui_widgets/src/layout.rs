use narui::{widget, Fragment, FragmentInner, WidgetContext};
pub use rutter_layout::{
    AbsolutePosition,
    Alignment,
    BoxConstraints,
    CrossAxisAlignment,
    EdgeInsets,
    FlexFit,
    MainAxisAlignment,
    MainAxisSize,
    Offset,
    StackFit,
};
use rutter_layout::{Align, Column, Flex, Flexible, Padding, Positioned, Row, SizedBox, Stack};


#[widget(
    cross_axis_alignment = rutter_layout::CrossAxisAlignment::Center,
    main_axis_alignment = rutter_layout::MainAxisAlignment::Center,
    main_axis_size = rutter_layout::MainAxisSize::Max
)]
pub fn column(
    children: narui::FragmentChildren,
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
    cross_axis_alignment = rutter_layout::CrossAxisAlignment::Center,
    main_axis_alignment = rutter_layout::MainAxisAlignment::Center,
    main_axis_size = rutter_layout::MainAxisSize::Max
)]
pub fn row(
    children: narui::FragmentChildren,
    cross_axis_alignment: rutter_layout::CrossAxisAlignment,
    main_axis_alignment: rutter_layout::MainAxisAlignment,
    main_axis_size: rutter_layout::MainAxisSize,
    context: &mut WidgetContext,
) -> FragmentInner {
    FragmentInner::Node {
        children,
        layout: Box::new(Row { cross_axis_alignment, main_axis_alignment, main_axis_size }),
        is_clipper: false,
    }
}

#[widget(flex = 1.0, fit = rutter_layout::FlexFit::Loose)]
pub fn flexible(
    children: Fragment,
    flex: f32,
    fit: rutter_layout::FlexFit,
    context: &mut WidgetContext,
) -> FragmentInner {
    FragmentInner::Node {
        children: narui::smallvec![children],
        layout: Box::new(Flexible { flex: Flex { flex, fit } }),
        is_clipper: false,
    }
}

#[widget(padding = rutter_layout::EdgeInsets::all(10.0))]
pub fn padding(
    children: Fragment,
    padding: rutter_layout::EdgeInsets,
    context: &mut WidgetContext,
) -> FragmentInner {
    FragmentInner::Node {
        children: narui::smallvec![children],
        layout: Box::new(Padding::new(padding)),
        is_clipper: false,
    }
}

#[widget(
    alignment = rutter_layout::Alignment::center(),
    factor_width = None,
    factor_height = None
)]
pub fn align(
    children: Fragment,
    alignment: rutter_layout::Alignment,
    factor_width: Option<f32>,
    factor_height: Option<f32>,
    context: &mut WidgetContext,
) -> FragmentInner {
    FragmentInner::Node {
        children: narui::smallvec![children],
        layout: Box::new(Align::fractional(alignment, factor_width, factor_height)),
        is_clipper: false,
    }
}

#[widget]
pub fn sized_box(
    children: Fragment,
    constraint: rutter_layout::BoxConstraints,
    context: &mut WidgetContext,
) -> FragmentInner {
    FragmentInner::Node {
        children: narui::smallvec![children],
        layout: Box::new(SizedBox::constrained(constraint)),
        is_clipper: false,
    }
}

#[widget(fit = rutter_layout::StackFit::Loose, alignment = rutter_layout::Alignment::center(), is_clipper = false)]
pub fn stack(
    children: narui::FragmentChildren,
    fit: rutter_layout::StackFit,
    alignment: rutter_layout::Alignment,
    is_clipper: bool,
    context: &mut WidgetContext,
) -> FragmentInner {
    FragmentInner::Node { children, layout: Box::new(Stack { fit, alignment }), is_clipper }
}

#[widget(pos = rutter_layout::AbsolutePosition::zero())]
pub fn positioned(
    children: Fragment,
    pos: rutter_layout::AbsolutePosition,
    context: &mut WidgetContext,
) -> FragmentInner {
    FragmentInner::Node {
        children: narui::smallvec![children],
        layout: Box::new(Positioned::new(pos)),
        is_clipper: false,
    }
}
