use narui::*;
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

#[widget(fit = StackFit::Loose, alignment = Alignment::center())]
pub fn stack(
    children: FragmentChildren,
    fit: StackFit,
    alignment: Alignment,
    context: &mut WidgetContext,
) -> FragmentInner {
    FragmentInner::Node { children, layout: Box::new(Stack { fit, alignment }), is_clipper: false }
}

#[widget(cross_axis_alignment = CrossAxisAlignment::Center, main_axis_alignment = MainAxisAlignment::Center, main_axis_size = MainAxisSize::Max)]
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

#[widget(cross_axis_alignment = CrossAxisAlignment::Center, main_axis_alignment = MainAxisAlignment::Center, main_axis_size = MainAxisSize::Max)]
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
        children: narui::smallvec![children],
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
        children: narui::smallvec![children],
        layout: Box::new(Padding::new(padding)),
        is_clipper: false,
    }
}

#[widget(alignment = Alignment::center(), factor_width = Default::default(), factor_height = Default::default())]
pub fn align(
    children: Fragment,
    alignment: Alignment,
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
    constraint: BoxConstraints,
    context: &mut WidgetContext,
) -> FragmentInner {
    FragmentInner::Node {
        children: narui::smallvec![children],
        layout: Box::new(SizedBox::constrained(constraint)),
        is_clipper: false,
    }
}


#[widget(pos = AbsolutePosition::zero())]
pub fn positioned(
    children: Fragment,
    pos: AbsolutePosition,
    context: &mut WidgetContext,
) -> FragmentInner {
    FragmentInner::Node {
        children: narui::smallvec![children],
        layout: Box::new(Positioned::new(pos)),
        is_clipper: false,
    }
}
