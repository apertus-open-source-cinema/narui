use crate::{
    Alignment,
    BoxConstraints,
    Dimension,
    Layout,
    LayoutableChild,
    LayoutableChildren,
    Offset,
    Positioner,
    Size,
};
use derivative::Derivative;
use std::any::Any;

#[derive(Debug, Clone, Copy, PartialEq)]
struct PositionedQuery;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Positioned {
    pub position: AbsolutePosition,
}

impl Positioned {
    pub fn new(position: AbsolutePosition) -> Self { Self { position } }

    pub fn get<'a>(child: &LayoutableChild<'a>) -> Option<&'a AbsolutePosition> {
        child.query(&PositionedQuery as &dyn Any).and_then(|v| <dyn Any>::downcast_ref(v))
    }
}

impl Layout for Positioned {
    fn layout(&self, constraint: BoxConstraints, children: LayoutableChildren) -> (Size, u32) {
        if children.len() > 1 {
            panic!("Positioned can have zero or one child but has {}", children.len())
        }
        if let Some(child) = children.into_iter().last() {
            let size = child.layout(constraint);
            child.set_pos(Offset::zero());
            child.set_z_index_offset(0);
            size
        } else {
            (constraint.constrain(Size::zero()), 0)
        }
    }

    fn query<'a>(
        &'a self,
        query: &dyn Any,
        children: LayoutableChildren<'a>,
    ) -> Option<&'a dyn Any> {
        if <dyn Any>::downcast_ref::<PositionedQuery>(query).is_some() {
            Some(&self.position)
        } else {
            assert!(children.len() <= 1);
            if let Some(child) = children.into_iter().last() {
                child.query(query)
            } else {
                None
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct AbsolutePosition {
    pub x: Dimension,
    pub y: Dimension,
}

impl Positioner for AbsolutePosition {
    fn position(&self, outer_size: Size, _inner_size: Size) -> Offset { self.position(outer_size) }
}

impl AbsolutePosition {
    pub fn zero() -> Self { Self { x: Dimension::Paxel(0.0), y: Dimension::Paxel(0.0) } }

    pub fn from_offset(offset: Offset) -> Self {
        Self { x: Dimension::Paxel(offset.x), y: Dimension::Paxel(offset.y) }
    }

    fn position(&self, outer_size: Size) -> Offset {
        let x = match self.x {
            Dimension::Paxel(x) => x,
            Dimension::Fraction(p) => outer_size.width * p,
        };

        let y = match self.y {
            Dimension::Paxel(y) => y,
            Dimension::Fraction(p) => outer_size.height * p,
        };

        Offset { x, y }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StackFit {
    Tight,
    Loose,
    Passthrough,
}

#[derive(Debug, Clone, Copy, PartialEq, Derivative)]
#[derivative(Default(new = "true"))]
pub struct Stack {
    #[derivative(Default(value = "StackFit::Loose"))]
    pub fit: StackFit,
    #[derivative(Default(value = "Alignment::center()"))]
    pub alignment: Alignment,
}

impl Stack {
    pub fn from(fit: StackFit, alignment: Alignment) -> Self { Self { fit, alignment } }
}

impl Layout for Stack {
    fn layout(&self, constraint: BoxConstraints, children: LayoutableChildren) -> (Size, u32) {
        let mut max_size = Size::zero();

        let non_positioned_constraint = match self.fit {
            StackFit::Tight => constraint.tighten(),
            StackFit::Loose => constraint.loosen(),
            StackFit::Passthrough => constraint,
        };

        for child in &children {
            if Positioned::get(&child).is_none() {
                let (size, num_z_index) = child.layout(non_positioned_constraint);
                max_size = max_size.max(size);
                child.set_z_index_offset(num_z_index);
            };
        }

        let mut z_index_offset = 0;
        let our_size = if matches!(self.fit, StackFit::Tight) {
            non_positioned_constraint.constrain(max_size)
        } else {
            constraint.constrain(max_size)
        };
        let positioned_constraint = BoxConstraints::tight_for(our_size).loosen();
        for child in &children {
            let num_z_index = if let Some(pos) = Positioned::get(&child) {
                let (_, num_z_index) = child.layout(positioned_constraint);
                child.set_pos(pos.position(our_size));
                num_z_index
            } else {
                child.set_pos(self.alignment.position(our_size, child.size()));
                child.z_index_offset()
            };
            child.set_z_index_offset(z_index_offset);
            z_index_offset += num_z_index;
        }

        (our_size, z_index_offset)
    }
}
