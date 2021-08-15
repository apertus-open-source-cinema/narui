use crate::heart::Context;
use crate::{Vec2, Key, PositionedRenderObject};
use crate::hooks::measure::MeasureError::{NoPreviousLayout, KeyNotFound, KeyAmbigous};

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
impl ContextMeasure for Context {
    fn measure_size(&self, key: Key) -> Result<Vec2, MeasureError> {
        let render_object = get_positioned_render_object(&self, key)?;
        Ok(render_object.rect.size)
    }

    fn measure_offset(&self, key1: Key, key2: Key) -> Result<Vec2, MeasureError> {
        let ro1 = get_positioned_render_object(&self, key1)?;
        let ro2 = get_positioned_render_object(&self, key2)?;
        Ok(ro2.rect.pos - ro1.rect.pos)
    }
}

fn get_positioned_render_object(context: &Context, key: Key) -> Result<PositionedRenderObject, MeasureError> {
    let layout = context.global.read().last_layout.clone().ok_or(NoPreviousLayout)?;
    let target_len = layout
        .iter()
        .filter(|x| x.key.starts_with(&key))
        .map(|x| x.key.len())
        .min()
        .ok_or(KeyNotFound)?;
    let render_objects: Vec<_> = layout
       .iter()
        .filter(|x| x.key.starts_with(&key) && x.key.len() == target_len)
        .collect();
    if render_objects.len() > 1 && !render_objects.iter().all(|x| x.rect == render_objects[0].rect) {
        dbg!(key, &layout, target_len, render_objects);
        return Err(KeyAmbigous)
    } else {
        Ok(render_objects[0].clone())
    }
}