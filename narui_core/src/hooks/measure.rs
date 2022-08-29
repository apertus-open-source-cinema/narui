use crate::{
    context::context::MaybeEvaluatedFragment::Evaluated,
    eval::layout::{LayoutTree, Physical, ScaleFactor},
    geom::{Rect, Vec2},
    CallbackContext,
    Fragment,
};

pub struct Measurement<T> {
    pub logical: T,
    pub physical: Physical<T>,
}

impl Measurement<Rect> {
    fn from_logical(v: Rect, scale_factor: ScaleFactor) -> Self {
        Measurement { logical: v, physical: v.to_physical(scale_factor) }
    }
}

impl Measurement<Vec2> {
    fn from_logical(v: Vec2, scale_factor: ScaleFactor) -> Self {
        Measurement { logical: v, physical: Physical::new(v * scale_factor.0) }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum MeasureError {
    NoPreviousLayout,
}

pub trait ContextMeasure {
    fn measure_size(&self, idx: Fragment) -> Result<Measurement<Vec2>, MeasureError>;
    fn measure_offset(
        &self,
        idx1: Fragment,
        idx2: Fragment,
    ) -> Result<Measurement<Vec2>, MeasureError>;
    fn measure(&self, idx: Fragment) -> Result<Measurement<Rect>, MeasureError>;
}
impl ContextMeasure for CallbackContext<'_> {
    fn measure_size(&self, idx: Fragment) -> Result<Measurement<Vec2>, MeasureError> {
        let rect = get_layout(self, idx)?;
        Ok(Measurement::<Vec2>::from_logical(rect.size, *self.scale_factor))
    }

    fn measure_offset(
        &self,
        idx1: Fragment,
        idx2: Fragment,
    ) -> Result<Measurement<Vec2>, MeasureError> {
        let ro1 = get_layout(self, idx1)?;
        let ro2 = get_layout(self, idx2)?;
        Ok(Measurement::<Vec2>::from_logical(ro2.pos - ro1.pos, *self.scale_factor))
    }

    fn measure(&self, idx: Fragment) -> Result<Measurement<Rect>, MeasureError> {
        Ok(Measurement::<Rect>::from_logical(get_layout(self, idx)?, *self.scale_factor))
    }
}

fn get_layout(context: &CallbackContext, idx: Fragment) -> Result<Rect, MeasureError> {
    let layout = context.layout;
    let frag = context.fragment_store.get(idx);
    match frag {
        Evaluated(frag) => {
            let rect = layout.get_positioned_logical(frag.layout_idx).0;
            // users do not know anything about logical vs physical currently, make this a
            // "normal" rect
            Ok(Rect { pos: rect.pos, size: rect.size })
        }
        _ => Err(MeasureError::NoPreviousLayout),
    }
}
