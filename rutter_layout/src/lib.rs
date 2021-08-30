mod types;
pub use types::{BoxConstraints, Offset, Size};

pub mod layouter;
pub use layouter::Layout;

pub mod layout_trait {
    pub use crate::layouter::{Layout, LayoutableChild, LayoutableChildren, TraitComparable};
}

pub mod layouts;

#[cfg(test)]
mod smoke_test;
