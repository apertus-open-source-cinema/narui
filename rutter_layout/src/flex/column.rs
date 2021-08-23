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
pub struct Column {
    pub cross_axis_alignment: CrossAxisAlignment,
    pub main_axis_alignment: MainAxisAlignment,
    pub main_axis_size: MainAxisSize,
}

impl Layoutable for Column {
    fn layout(&self, constraint: BoxConstraints, children: LayoutableChildren) -> Size {
        let orig_constraint = constraint;
        let constraint = constraint.loosen_width();

        let non_flex_constraint = constraint.with_unbounded_height();
        let mut max_width = 0.0f32;
        let mut bounded_height = 0.0;
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
                    max_width = max_width.max(size.width);
                    bounded_height += size.height;
                }
            }
        }

        assert!(
            constraint.height_is_bounded()
                || (!any_tight && matches!(self.main_axis_size, MainAxisSize::Min))
        );

        let unit_flex = if total_flex != 0.0 {
            (constraint.max_height - bounded_height) / total_flex
        } else {
            0.0
        };
        let total_spacing = if matches!(self.main_axis_size, MainAxisSize::Min) {
            constraint.min_height.max(bounded_height).min(constraint.min_height) - bounded_height
        } else {
            constraint.max_height - bounded_height
        };
        let unit_flex = unit_flex.max(0.0);
        let total_spacing = total_spacing.max(0.0);

        let mut actual_flex_height = 0.0;

        for child in children.into_iter() {
            if let Some(Flex { flex, fit }) = Flexible::get(&child) {
                let flex_space = flex * unit_flex;
                let constraint = match fit {
                    FlexFit::Tight => constraint.with_tight_height(flex_space),
                    FlexFit::Loose => constraint.with_loose_height(flex_space),
                };

                let size = child.layout(constraint);
                actual_flex_height += size.height;
                max_width = max_width.max(size.width);
            }
        }

        let max_width = max_width.clamp(orig_constraint.min_width, orig_constraint.max_width);
        let total_spacing = (total_spacing - actual_flex_height).max(0.0);
        let total_height = bounded_height + actual_flex_height;

        let (space_before, space_between) =
            self.main_axis_alignment.spacing_for(total_spacing, children.len());

        let mut current_height = space_before;

        for child in children.into_iter() {
            let size = child.size();
            child.set_pos(Offset {
                x: self.cross_axis_alignment.spacing_for(max_width, size.width),
                y: current_height,
            });
            current_height += space_between + size.height;
        }

        constraint.constrain(Size { height: total_height + total_spacing, width: max_width })
    }
}
