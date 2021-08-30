use crate::{
    context::context::MaybeEvaluatedFragment::Evaluated,
    eval::layout::LayoutTree,
    geom::{Rect, Vec2},
    CallbackContext,
    Fragment,
};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum MeasureError {
    NoPreviousLayout,
}

pub trait ContextMeasure {
    fn measure_size(&self, idx: Fragment) -> Result<Vec2, MeasureError>;
    fn measure_offset(&self, idx1: Fragment, idx2: Fragment) -> Result<Vec2, MeasureError>;
}
impl ContextMeasure for CallbackContext<'_> {
    fn measure_size(&self, idx: Fragment) -> Result<Vec2, MeasureError> {
        let rect = get_layout(self, idx)?;
        Ok(rect.size)
    }

    fn measure_offset(&self, idx1: Fragment, idx2: Fragment) -> Result<Vec2, MeasureError> {
        let ro1 = get_layout(self, idx1)?;
        let ro2 = get_layout(self, idx2)?;
        Ok(ro2.pos - ro1.pos)
    }
}

fn get_layout(context: &CallbackContext, idx: Fragment) -> Result<Rect, MeasureError> {
    let layout = context.layout;
    let frag = context.fragment_store.get(idx);
    match frag {
        Evaluated(frag) => Ok(layout.get_positioned(frag.layout_idx).0),
        _ => Err(MeasureError::NoPreviousLayout),
    }
}
