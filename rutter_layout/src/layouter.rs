use crate::{
    util::{BiMap, Idx, VecWithHoles},
    BoxConstraints,
    Offset,
    Size,
};
use std::{any::Any, cell::Cell, hash::Hash, ops::Deref};
use tinyset::Set64;

pub trait Layoutable: std::fmt::Debug {
    fn uses_child_size(&self) -> bool { true }

    fn layout(&self, constraint: BoxConstraints, children: LayoutableChildren) -> Size;

    fn query<'a>(
        &'a self,
        _query: &dyn Any,
        _children: LayoutableChildren<'a>,
    ) -> Option<&'a dyn Any> {
        None
    }
}

#[derive(Debug)]
pub struct Layouter<Key, T> {
    key_to_idx: BiMap<Key, Idx>,
    nodes: VecWithHoles<PositionedNode<T>>,
}

impl<Key, T> Layouter<Key, T> {
    pub fn new() -> Self { Self { key_to_idx: BiMap::new(), nodes: VecWithHoles::new() } }
}

impl<Key: Hash + Eq + Clone, T: Deref<Target = dyn Layoutable> + std::fmt::Debug> Layouter<Key, T> {
    pub fn set_node(&mut self, key: Key, layoutable: T) {
        match self.key_to_idx.get_left(&key) {
            Some(idx) => {
                self.nodes[*idx].obj = layoutable;
                self.propagate_dirty(*idx);
            }
            None => {
                let idx = self.nodes.add(PositionedNode::new(layoutable));
                self.key_to_idx.insert(key, Idx::new(idx));
            }
        }
    }

    fn propagate_dirty(&self, idx: Idx) {
        let mut next = Some(idx);
        let mut first = true;

        while let Some(idx) = next {
            let node = &self.nodes[idx];
            if (node.any_dirty_children.get() || !node.obj.uses_child_size()) && !first {
                break;
            }

            node.any_dirty_children.set(true);

            next = node.parent;
            first = false;
        }
    }

    pub fn set_children(&mut self, key: Key, children: &[Key]) {
        let parent_idx = *self.key_to_idx.get_left(&key).unwrap();

        let mut old = Set64::new();
        let mut new = Set64::new();

        let mut old_child_ptr = self.nodes[parent_idx].child;
        while let Some(old_child_idx) = old_child_ptr {
            old.insert(old_child_idx.get());
            old_child_ptr = self.nodes[old_child_idx].next_sibling;
        }

        if !children.is_empty() {
            let new_child_idx = *self.key_to_idx.get_left(&children[0]).unwrap();
            self.nodes[parent_idx].child = Some(new_child_idx);
            let mut last_child_idx = new_child_idx;

            for new_child in &children[1..] {
                let new_child_idx = *self.key_to_idx.get_left(&new_child).unwrap();
                new.insert(new_child_idx.get());

                self.nodes[last_child_idx].next_sibling = Some(new_child_idx);

                assert!(
                    (self.nodes[last_child_idx].parent == None)
                        || (self.nodes[last_child_idx].parent == Some(parent_idx))
                );
                self.nodes[last_child_idx].parent = Some(parent_idx);

                last_child_idx = new_child_idx;
            }

            self.nodes[last_child_idx].next_sibling = None;
            assert!(
                (self.nodes[last_child_idx].parent == None)
                    || (self.nodes[last_child_idx].parent == Some(parent_idx))
            );
            self.nodes[last_child_idx].parent = Some(parent_idx);
        } else {
            self.nodes[parent_idx].next_sibling = None;
        }

        self.nodes[parent_idx].num_children = children.len();

        for o in old {
            if !new.contains(o) {
                self.remove(Idx::new(o));
            }
        }

        self.propagate_dirty(parent_idx);
    }

    fn remove(&mut self, idx: Idx) {
        self.nodes.remove(idx.get());
        self.key_to_idx.remove_right(idx);
    }

    pub fn do_layout(&mut self, constraints: BoxConstraints, root_pos: Offset, root: Key) {
        let idx = *self.key_to_idx.get_left(&root).unwrap();
        let nodes_wrapper = &self.nodes.iter_raw().map(|v| v as _).collect::<Vec<_>>()[..];

        let child = LayoutableChild::new(idx, nodes_wrapper);
        child.layout(constraints);
        // TODO(robin): is there any better thing to do here?
        child.set_pos(Offset::zero());

        self.propagate_abs_pos(idx, root_pos, true);
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

    pub fn get_layout(&self, key: Key) -> (Offset, Size) {
        let idx = self.key_to_idx.get_left(&key).unwrap();
        let node = &self.nodes[idx];
        (node.abs_pos.get().unwrap(), node.size.get().unwrap())
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
            any_dirty_children: Cell::new(true),
            input_constraint: Cell::new(None),
            size: Cell::new(None),
            pos: Cell::new(None),
            abs_pos: Cell::new(None),
            dirty_abs_pos: Cell::new(true),
            any_dirty_child_abs_pos: Cell::new(true),
        }
    }
}

impl<T: Deref<Target = dyn Layoutable> + std::fmt::Debug> PositionedNodeT for PositionedNode<T> {
    fn obj(&self) -> &dyn Layoutable { &*self.obj }

    fn child(&self) -> Option<Idx> { self.child }

    fn num_children(&self) -> usize { self.num_children }

    fn next_sibling(&self) -> Option<Idx> { self.next_sibling }

    fn parent(&self) -> Option<Idx> { self.parent }

    fn any_dirty_children(&self) -> bool { self.any_dirty_children.get() }

    fn set_any_dirty_children(&self, val: bool) { self.any_dirty_children.set(val) }

    fn input_constraints(&self) -> Option<BoxConstraints> { self.input_constraint.get() }

    fn set_input_constraints(&self, val: Option<BoxConstraints>) { self.input_constraint.set(val) }

    fn size(&self) -> Option<Size> { self.size.get() }

    fn set_size(&self, val: Option<Size>) { self.size.set(val) }

    fn pos(&self) -> Option<Offset> { self.pos.get() }

    fn set_pos(&self, val: Option<Offset>) { self.pos.set(val) }

    fn dirty_abs_pos(&self) -> bool { self.dirty_abs_pos.get() }

    fn set_dirty_abs_pos(&self, val: bool) { self.dirty_abs_pos.set(val) }

    fn any_dirty_child_abs_pos(&self) -> bool { self.any_dirty_child_abs_pos.get() }

    fn set_any_dirty_child_abs_pos(&self, val: bool) { self.any_dirty_child_abs_pos.set(val) }
}

trait PositionedNodeT: std::fmt::Debug {
    fn obj(&self) -> &dyn Layoutable;

    fn child(&self) -> Option<Idx>;
    fn num_children(&self) -> usize;
    fn next_sibling(&self) -> Option<Idx>;
    fn parent(&self) -> Option<Idx>;

    fn any_dirty_children(&self) -> bool;
    fn set_any_dirty_children(&self, val: bool);

    fn input_constraints(&self) -> Option<BoxConstraints>;
    fn set_input_constraints(&self, val: Option<BoxConstraints>);

    fn size(&self) -> Option<Size>;
    fn set_size(&self, val: Option<Size>);

    fn pos(&self) -> Option<Offset>;
    fn set_pos(&self, val: Option<Offset>);

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

    pub fn uses_child_size(&self) -> bool { self.child.obj().uses_child_size() }

    pub fn layout(&self, constraint: BoxConstraints) -> Size {
        if !self.child.any_dirty_children() && Some(constraint) == self.child.input_constraints() {
            if let Some(size) = self.child.size() {
                return size;
            }
        }

        let size =
            self.child.obj().layout(constraint, LayoutableChildren::new(self.child, self.nodes));
        self.child.set_input_constraints(Some(constraint));
        self.child.set_any_dirty_children(false);
        self.child.set_size(Some(size));
        size
    }

    pub fn set_pos(&self, pos: Offset) {
        let old_pos = self.child.pos();
        self.child.set_pos(Some(pos));

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
