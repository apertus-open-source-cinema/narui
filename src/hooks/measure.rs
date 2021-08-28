use crate::{
    hooks::measure::MeasureError::KeyNotFound,
    CallbackContext,
    Key,
    LayoutTree,
    Rect,
    Vec2,
};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum MeasureError {
    NoPreviousLayout,
    KeyNotFound,
    KeyAmbigous,
}

pub trait ContextMeasure {
    fn measure_size(&self, key: Key) -> Result<Vec2, MeasureError>;
    fn measure_offset(&self, key1: Key, key2: Key) -> Result<Vec2, MeasureError>;
}
impl ContextMeasure for CallbackContext<'_> {
    fn measure_size(&self, key: Key) -> Result<Vec2, MeasureError> {
        let rect = get_layout(self, key)?;
        Ok(rect.size)
    }

    fn measure_offset(&self, key1: Key, key2: Key) -> Result<Vec2, MeasureError> {
        let ro1 = get_layout(self, key1)?;
        let ro2 = get_layout(self, key2)?;
        Ok(ro2.pos - ro1.pos)
    }
}

fn get_layout(context: &CallbackContext, key: Key) -> Result<Rect, MeasureError> {
    let layout = context.layout;
    context
        .key_to_fragment
        .get(&key)
        .map(|frag| layout.get_positioned(frag.borrow().idx).0)
        .ok_or(KeyNotFound)
}
