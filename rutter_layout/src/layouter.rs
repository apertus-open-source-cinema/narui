use crate::{BoxConstraints, Offset, Size};
use derivative::Derivative;
use freelist::FreeList;
pub use freelist::Idx;
use std::{any::Any, cell::Cell, fmt::Debug, ops::Deref};


pub trait TraitComparable: std::fmt::Debug {
    fn as_any(&self) -> &dyn Any;

    fn as_trait_comparable(&self) -> &dyn TraitComparable;

    fn eq(&self, other: &dyn TraitComparable) -> bool;
}

impl<T: 'static + PartialEq + std::fmt::Debug> TraitComparable for T {
    fn as_any(&self) -> &dyn Any { self }

    fn as_trait_comparable(&self) -> &dyn TraitComparable { self }

    fn eq(&self, other: &dyn TraitComparable) -> bool {
        other.as_any().downcast_ref::<T>().map_or(false, |other| self == other)
    }
}

pub trait Layout: std::fmt::Debug + TraitComparable {
    fn eq(&self, other: &dyn Layout) -> bool {
        TraitComparable::eq(self, other.as_trait_comparable())
    }

    // gets constraints, returns a size and the number of used z_indices
    fn layout(&self, constraint: BoxConstraints, children: LayoutableChildren) -> (Size, u32);

    fn query<'a>(
        &'a self,
        _query: &dyn Any,
        _children: LayoutableChildren<'a>,
    ) -> Option<&'a dyn Any> {
        None
    }
}

pub struct LayoutItem<'a, T> {
    pub size: Size,
    pub pos: Offset,
    pub idx: Idx,
    pub z_index: u32,
    pub obj: &'a T,
}

impl<'a, T> LayoutItem<'a, T> {
    fn new(layouter: &'a Layouter<T>, idx: Idx, z_index: u32) -> Self {
        let node = &layouter.nodes[idx];

        Self {
            size: node.size.get().unwrap(),
            pos: node.abs_pos.get().unwrap(),
            obj: &node.obj,
            z_index,
            idx,
        }
    }
}

struct LayoutIter<'a, T> {
    layouter: &'a Layouter<T>,
    next_pos: Option<Idx>,
    parent_z_index: u32,
}

impl<'a, T: Debug> LayoutIter<'a, T> {
    fn new(layouter: &'a Layouter<T>, top: Idx) -> Self {
        let mut bottom_left = top;
        let mut parent_z_index = 0;
        while let Some(child) = layouter.nodes[bottom_left].child {
            parent_z_index += layouter.nodes[bottom_left].z_index_offset.get().unwrap();
            bottom_left = child;
        }
        Self { layouter, next_pos: Some(bottom_left), parent_z_index }
    }
}

impl<'a, T: std::fmt::Debug> Iterator for LayoutIter<'a, T> {
    type Item = (Idx, u32);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(current_pos) = self.next_pos {
            let current = &self.layouter.nodes[current_pos];
            let z_index = self.parent_z_index + current.z_index_offset.get().unwrap();
            match current.next_sibling {
                Some(idx) => {
                    let mut next = idx;

                    while let Some(child) = self.layouter.nodes[next].child {
                        self.parent_z_index +=
                            self.layouter.nodes[next].z_index_offset.get().unwrap();
                        next = child;
                    }

                    self.next_pos = Some(next);
                }
                None => {
                    self.next_pos = current.parent;
                    if let Some(idx) = current.parent {
                        self.parent_z_index -=
                            &self.layouter.nodes[idx].z_index_offset.get().unwrap();
                    }
                }
            }
            Some((current_pos, z_index))
        } else {
            None
        }
    }
}

#[derive(Debug, Derivative)]
#[derivative(Default(bound = "", new = "true"))]
pub struct Layouter<T> {
    nodes: FreeList<PositionedNode<T>>,
}

impl<T: Deref<Target = dyn Layout> + std::fmt::Debug> Layouter<T> {
    pub fn add_node(&mut self, layoutable: T) -> Idx {
        self.nodes.add(PositionedNode::new(layoutable))
    }

    pub fn set_node(&mut self, idx: Idx, layoutable: T) {
        let dirty = !Layout::eq(&*self.nodes[idx].obj, &*layoutable);
        // dbg!(dirty, &self.nodes[*idx].obj, &layoutable);
        self.nodes[idx].obj = layoutable;
        if dirty {
            self.propagate_dirty(idx);
        }
    }

    fn propagate_dirty(&self, idx: Idx) {
        let mut next = Some(idx);

        while let Some(idx) = next {
            let node = &self.nodes[idx];
            if node.any_dirty_children.get() {
                break;
            }

            // dbg!("setting dirty", node);
            node.any_dirty_children.set(true);

            next = node.parent;
        }
    }

    pub fn set_children<'a>(&mut self, parent_idx: Idx, mut children: impl Iterator<Item = Idx>) {
        let mut len = 0;
        let mut any_child_dirty = false;

        if let Some(new_child_idx) = children.next() {
            len = 1;
            self.nodes[parent_idx].child = Some(new_child_idx);
            let mut last_child_idx = new_child_idx;

            for new_child_idx in children {
                len += 1;

                let last = &mut self.nodes[last_child_idx];
                any_child_dirty = any_child_dirty || last.any_dirty_children.get();

                last.next_sibling = Some(new_child_idx);

                assert!((last.parent == None) || (last.parent == Some(parent_idx)));
                last.parent = Some(parent_idx);

                last_child_idx = new_child_idx;
            }

            let last = &mut self.nodes[last_child_idx];
            any_child_dirty = any_child_dirty || last.any_dirty_children.get();
            last.next_sibling = None;
            assert!((last.parent == None) || (last.parent == Some(parent_idx)));
            last.parent = Some(parent_idx);
        } else {
            self.nodes[parent_idx].child = None;
        }

        let dirty = (self.nodes[parent_idx].num_children != len) || any_child_dirty;
        self.nodes[parent_idx].num_children = len;

        if dirty {
            self.propagate_dirty(parent_idx);
        }
    }

    pub fn remove(&mut self, idx: Idx) { self.nodes.remove(idx); }

    pub fn do_layout(&mut self, constraints: BoxConstraints, root_pos: Offset, idx: Idx) {
        let nodes_wrapper =
            unsafe { &self.nodes.iter_raw().map(|v| v as _).collect::<Vec<_>>()[..] };

        let child = LayoutableChild::new(idx, nodes_wrapper);
        child.layout(constraints);
        // TODO(robin): is there any better thing to do here?
        child.set_pos(Offset::zero());
        child.set_z_index_offset(1);

        self.propagate_abs_pos(idx, root_pos, true);
    }

    pub fn iter(&self, idx: Idx) -> impl Iterator<Item = LayoutItem<T>> {
        LayoutIter::new(self, idx).map(move |(idx, z_index)| LayoutItem::new(self, idx, z_index))
    }

    fn propagate_abs_pos(&self, root: Idx, offset: Offset, dirty: bool) {
        let node = &self.nodes[root];

        let (dirty, my_offset) = if node.dirty_abs_pos() || dirty {
            let rel_offset = node.pos.get().unwrap();
            let new_abs_pos = rel_offset + offset;
            if Some(new_abs_pos) != node.abs_pos.get() {
                node.abs_pos.set(Some(new_abs_pos));
                (true, new_abs_pos)
            } else {
                (false, new_abs_pos)
            }
        } else {
            (false, node.abs_pos.get().unwrap())
        };

        if dirty || node.any_dirty_child_abs_pos.get() {
            let mut child = node.child;
            while let Some(c) = child {
                self.propagate_abs_pos(c, my_offset, dirty);
                child = self.nodes[c].next_sibling;
            }
        }

        node.any_dirty_child_abs_pos.set(false);
        node.dirty_abs_pos.set(false);
    }

    pub fn get_layout(&self, idx: Idx) -> (Offset, Size, &T) {
        let node = &self.nodes[idx];
        (node.abs_pos.get().unwrap(), node.size.get().unwrap(), &node.obj)
    }
}

#[derive(Debug)]
struct PositionedNode<T> {
    obj: T,
    child: Option<Idx>,
    num_children: usize,
    next_sibling: Option<Idx>,
    parent: Option<Idx>,
    any_dirty_children: Cell<bool>,
    input_constraint: Cell<Option<BoxConstraints>>,
    size: Cell<Option<Size>>,
    num_z_index: Cell<Option<u32>>,
    z_index_offset: Cell<Option<u32>>,
    pos: Cell<Option<Offset>>,
    abs_pos: Cell<Option<Offset>>,
    dirty_abs_pos: Cell<bool>,
    any_dirty_child_abs_pos: Cell<bool>,
}

impl<T> PositionedNode<T> {
    fn new(obj: T) -> PositionedNode<T> {
        PositionedNode {
            obj,
            child: None,
            num_children: 0,
            next_sibling: None,
            parent: None,
            any_dirty_children: Cell::new(false),
            input_constraint: Cell::new(None),
            size: Cell::new(None),
            num_z_index: Cell::new(None),
            z_index_offset: Cell::new(None),
            pos: Cell::new(None),
            abs_pos: Cell::new(None),
            dirty_abs_pos: Cell::new(true),
            any_dirty_child_abs_pos: Cell::new(true),
        }
    }
}

impl<T: Deref<Target = dyn Layout> + std::fmt::Debug> PositionedNodeT for PositionedNode<T> {
    fn obj(&self) -> &dyn Layout { &*self.obj }

    fn child(&self) -> Option<Idx> { self.child }

    fn num_children(&self) -> usize { self.num_children }

    fn next_sibling(&self) -> Option<Idx> { self.next_sibling }

    fn parent(&self) -> Option<Idx> { self.parent }

    fn any_dirty_children(&self) -> bool { self.any_dirty_children.get() }

    fn set_any_dirty_children(&self, val: bool) { self.any_dirty_children.set(val) }

    fn input_constraints(&self) -> Option<BoxConstraints> { self.input_constraint.get() }

    fn set_input_constraints(&self, val: BoxConstraints) { self.input_constraint.set(Some(val)) }

    fn size(&self) -> Option<Size> { self.size.get() }

    fn set_size(&self, val: Size) { self.size.set(Some(val)) }

    fn num_z_index(&self) -> Option<u32> { self.num_z_index.get() }

    fn set_num_z_index(&self, val: u32) { self.num_z_index.set(Some(val)) }

    fn z_index_offset(&self) -> Option<u32> { self.z_index_offset.get() }

    fn set_z_index_offset(&self, val: u32) { self.z_index_offset.set(Some(val)) }

    fn pos(&self) -> Option<Offset> { self.pos.get() }

    fn set_pos(&self, val: Offset) { self.pos.set(Some(val)) }

    fn dirty_abs_pos(&self) -> bool { self.dirty_abs_pos.get() }

    fn set_dirty_abs_pos(&self, val: bool) { self.dirty_abs_pos.set(val) }

    fn any_dirty_child_abs_pos(&self) -> bool { self.any_dirty_child_abs_pos.get() }

    fn set_any_dirty_child_abs_pos(&self, val: bool) { self.any_dirty_child_abs_pos.set(val) }
}

trait PositionedNodeT: std::fmt::Debug {
    fn obj(&self) -> &dyn Layout;

    fn child(&self) -> Option<Idx>;
    fn num_children(&self) -> usize;
    fn next_sibling(&self) -> Option<Idx>;
    fn parent(&self) -> Option<Idx>;

    fn any_dirty_children(&self) -> bool;
    fn set_any_dirty_children(&self, val: bool);

    fn input_constraints(&self) -> Option<BoxConstraints>;
    fn set_input_constraints(&self, val: BoxConstraints);

    fn size(&self) -> Option<Size>;
    fn set_size(&self, val: Size);

    fn num_z_index(&self) -> Option<u32>;
    fn set_num_z_index(&self, val: u32);

    fn z_index_offset(&self) -> Option<u32>;
    fn set_z_index_offset(&self, val: u32);

    fn pos(&self) -> Option<Offset>;
    fn set_pos(&self, val: Offset);

    fn dirty_abs_pos(&self) -> bool;
    fn set_dirty_abs_pos(&self, val: bool);

    fn any_dirty_child_abs_pos(&self) -> bool;
    fn set_any_dirty_child_abs_pos(&self, val: bool);
}

#[derive(Debug)]
pub struct LayoutableChildren<'a> {
    nodes: &'a [&'a dyn PositionedNodeT],
    first: Option<Idx>,
    len: usize,
}

impl<'a> LayoutableChildren<'a> {
    fn new(parent: &'a dyn PositionedNodeT, nodes: &'a [&'a dyn PositionedNodeT]) -> Self {
        let first = parent.child();
        let len = parent.num_children();

        Self { nodes, first, len }
    }

    pub fn len(&self) -> usize { self.len }

    pub fn is_empty(&self) -> bool { self.len == 0 }
}

#[derive(Debug)]
pub struct LayoutableChildrenIter<'a> {
    nodes: &'a [&'a dyn PositionedNodeT],
    pos: Option<Idx>,
}

impl<'a> Iterator for LayoutableChildrenIter<'a> {
    type Item = LayoutableChild<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.pos.map(|pos| {
            let node = self.nodes[pos.get()];
            self.pos = node.next_sibling();
            LayoutableChild { child: node, nodes: self.nodes }
        })
    }
}

#[derive(Debug)]
pub struct LayoutableChild<'a> {
    nodes: &'a [&'a dyn PositionedNodeT],
    child: &'a dyn PositionedNodeT,
}

impl<'a> LayoutableChild<'a> {
    fn new(idx: Idx, nodes: &'a [&'a dyn PositionedNodeT]) -> Self {
        Self { child: nodes[idx.get()], nodes }
    }

    pub fn layout(&self, constraint: BoxConstraints) -> (Size, u32) {
        // dbg!("before_layout", self.child.obj(), self.child.any_dirty_children(),
        // constraint, self.child.input_constraints(), self.child.size());
        if !self.child.any_dirty_children() && Some(constraint) == self.child.input_constraints() {
            if let (Some(size), Some(num_z_index)) = (self.child.size(), self.child.num_z_index()) {
                return (size, num_z_index);
            }
        }

        let (size, num_z_index) =
            self.child.obj().layout(constraint, LayoutableChildren::new(self.child, self.nodes));
        self.child.set_input_constraints(constraint);
        self.child.set_any_dirty_children(false);
        self.child.set_size(size);
        self.child.set_num_z_index(num_z_index);
        // dbg!("after_layout", self.child.obj(), self.child.any_dirty_children(),
        // constraint, self.child.input_constraints(), self.child.size());
        (size, num_z_index)
    }

    pub fn set_pos(&self, pos: Offset) {
        let old_pos = self.child.pos();
        self.child.set_pos(pos);

        if old_pos != Some(pos) {
            self.child.set_dirty_abs_pos(true);

            let mut parent = self.child.parent();
            while let Some(parent_idx) = parent {
                let parent_node = self.nodes[parent_idx.get()];
                parent_node.set_any_dirty_child_abs_pos(true);
                parent = parent_node.parent();
            }
        }
    }

    pub fn set_z_index_offset(&self, idx: u32) { self.child.set_z_index_offset(idx); }

    pub fn z_index_offset(&self) -> u32 { self.child.z_index_offset().unwrap() }

    // returns the size determined by the last layout call
    // you have to call layout first if you want it to match your constraints
    pub fn size(&self) -> Size { self.child.size().unwrap() }

    pub fn query(&self, query: &dyn Any) -> Option<&'a dyn Any> {
        self.child.obj().query(query, LayoutableChildren::new(self.child, self.nodes))
    }
}

impl<'a, 'b> IntoIterator for &'a LayoutableChildren<'b> {
    type Item = LayoutableChild<'b>;
    type IntoIter = LayoutableChildrenIter<'b>;

    fn into_iter(self) -> Self::IntoIter {
        LayoutableChildrenIter { nodes: self.nodes, pos: self.first }
    }
}
