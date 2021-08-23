use crate::{
    BoxConstraints,
    CrossAxisAlignment,
    Flex,
    FlexFit,
    Flexible,
    Layoutable,
    LayoutableChildren,
    MainAxisAlignment,
    MainAxisSize,
    Offset,
    Size,
};

#[derive(Debug)]
pub struct Row {
    pub cross_axis_alignment: CrossAxisAlignment,
    pub main_axis_alignment: MainAxisAlignment,
    pub main_axis_size: MainAxisSize,
}

impl Layoutable for Row {
    fn layout(&self, constraint: BoxConstraints, children: LayoutableChildren) -> Size {
        let orig_constraint = constraint;
        let constraint = constraint.loosen_height();

        let non_flex_constraint = constraint.with_unbounded_width();
        let mut max_height = 0.0f32;
        let mut bounded_width = 0.0;
        let mut total_flex = 0.0;
        let mut any_tight = false;

        for child in children.into_iter() {
            match Flexible::get(&child) {
                Some(Flex { flex, fit }) => {
                    total_flex += flex;
                    any_tight = any_tight || matches!(fit, FlexFit::Tight);
                }
                None => {
                    let size = child.layout(non_flex_constraint);
                    max_height = max_height.max(size.height);
                    bounded_width += size.width;
                }
            }
        }

        assert!(
            constraint.width_is_bounded()
                || (!any_tight && matches!(self.main_axis_size, MainAxisSize::Min))
        );

        let unit_flex = if total_flex != 0.0 {
            (constraint.max_width - bounded_width) / total_flex
        } else {
            0.0
        };
        let total_spacing = if matches!(self.main_axis_size, MainAxisSize::Min) {
            bounded_width.clamp(constraint.min_width, constraint.max_height) - bounded_width
        } else {
            constraint.max_width - bounded_width
        };
        let unit_flex = unit_flex.max(0.0);
        let total_spacing = total_spacing.max(0.0);

        let mut actual_flex_width = 0.0;

        for child in children.into_iter() {
            if let Some(Flex { flex, fit }) = Flexible::get(&child) {
                let flex_space = flex * unit_flex;
                let constraint = match fit {
                    FlexFit::Tight => constraint.with_tight_width(flex_space),
                    FlexFit::Loose => constraint.with_loose_width(flex_space),
                };

                let size = child.layout(constraint);
                actual_flex_width += size.width;
                max_height = max_height.max(size.height);
            }
        }

        let max_height = max_height.clamp(orig_constraint.min_height, orig_constraint.max_height);
        let total_spacing = (total_spacing - actual_flex_width).max(0.0);
        let total_width = bounded_width + actual_flex_width;

        let (space_before, space_between) =
            self.main_axis_alignment.spacing_for(total_spacing, children.len());

        let mut current_width = space_before;

        for child in children.into_iter() {
            let size = child.size();
            child.set_pos(Offset {
                x: self.cross_axis_alignment.spacing_for(max_height, size.height),
                y: current_width,
            });
            current_width += space_between + size.width;
        }

        constraint.constrain(Size { height: max_height, width: total_width + total_spacing })
    }
}
