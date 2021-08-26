use crate::{
    util::{BiMap, Idx, VecWithHoles},
    BoxConstraints,
    Offset,
    Size,
};
use std::{
    any::Any,
    cell::Cell,
    collections::hash_map::DefaultHasher,
    hash::{BuildHasher, Hash},
    ops::Deref,
};


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

    fn layout(&self, constraint: BoxConstraints, children: LayoutableChildren) -> Size;

    fn query<'a>(
        &'a self,
        _query: &dyn Any,
        _children: LayoutableChildren<'a>,
    ) -> Option<&'a dyn Any> {
        None
    }
}

pub struct LayoutItemWithObj<'a, T> {
    pub size: Size,
    pub pos: Offset,
    pub obj: &'a T
}

impl<'a, T> LayoutItemWithObj<'a, T> {
    fn new<Key, H: BuildHasher>(layouter: &'a Layouter<Key, T, H>, idx: Idx) -> Self {
        let node = &layouter.nodes[idx.get()];

        Self {
            size: node.size.get().unwrap(),
            pos: node.abs_pos.get().unwrap(),
            obj: &node.obj
        }
    }
}

pub struct LayoutItem<'a, Key> {
    pub size: Size,
    pub pos: Offset,
    pub key: &'a Key,
}

impl<'a, Key: Hash + Eq + Clone> LayoutItem<'a, Key> {
    fn new<T, H: BuildHasher>(layouter: &'a Layouter<Key, T, H>, idx: Idx) -> Self {
        let node = &layouter.nodes[idx.get()];

        Self {
            size: node.size.get().unwrap(),
            pos: node.abs_pos.get().unwrap(),
            key: layouter.key_to_idx.get_right(&idx).unwrap(),
        }
    }
}

struct LayoutIter<'a, Key, T, H> {
    layouter: &'a Layouter<Key, T, H>,
    next_pos: Option<Idx>,
}

impl<'a, Key, T, H> LayoutIter<'a, Key, T, H> {
    fn new(layouter: &'a Layouter<Key, T, H>, top: Idx) -> Self {
        let mut bottom_left = top;
        while let Some(child) = layouter.nodes[bottom_left.get()].child {
            bottom_left = child;
        }
        Self { layouter, next_pos: Some(bottom_left) }
    }
}

impl<'a, Key: Hash + Eq + Clone, T, H: BuildHasher> Iterator for LayoutIter<'a, Key, T, H> {
    type Item = Idx;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(current_pos) = self.next_pos {
            let current = &self.layouter.nodes[current_pos];
            match current.next_sibling {
                Some(idx) => {
                    let mut next = idx;

                    while let Some(child) = self.layouter.nodes[next.get()].child {
                        next = child;
                    }

                    self.next_pos = Some(next);
                }
                None => self.next_pos = current.parent,
            }
            Some(current_pos)
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct Layouter<Key, T, Hasher = DefaultHasher> {
    key_to_idx: BiMap<Key, Idx, Hasher>,
    nodes: VecWithHoles<PositionedNode<T>>,
}

impl<Key, T, H: Default> Layouter<Key, T, H> {
    pub fn new() -> Self { Self { key_to_idx: BiMap::new(), nodes: VecWithHoles::new() } }
}

impl<Key: Hash + Eq + Clone, T: Deref<Target = dyn Layout> + std::fmt::Debug, H: BuildHasher>
    Layouter<Key, T, H>
{
    pub fn set_node(&mut self, key: &Key, layoutable: T) {
        match self.key_to_idx.get_left(key) {
            Some(idx) => {
                let dirty = !Layout::eq(&*self.nodes[*idx].obj, &*layoutable);
                // dbg!(dirty, &self.nodes[*idx].obj, &layoutable);
                self.nodes[*idx].obj = layoutable;
                if dirty {
                    self.propagate_dirty(*idx);
                }
            }
            None => {
                let idx = self.nodes.add(PositionedNode::new(layoutable));
                self.key_to_idx.insert(key.clone(), Idx::new(idx));
            }
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

    pub fn set_children<'a>(&mut self, key: &Key, mut children: impl Iterator<Item = &'a Key>)
    where
        Key: 'a,
    {
        let parent_idx = *self.key_to_idx.get_left(key).unwrap();
        let mut len = 0;
        let mut any_child_dirty = false;

        if let Some(first_child) = children.next() {
            len = 1;
            let new_child_idx = *self.key_to_idx.get_left(&first_child).unwrap();
            self.nodes[parent_idx].child = Some(new_child_idx);
            let mut last_child_idx = new_child_idx;

            for new_child in children {
                len += 1;
                let new_child_idx = *self.key_to_idx.get_left(&new_child).unwrap();

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

    pub fn remove(&mut self, key: &Key) {
        let (_key, idx) = self.key_to_idx.remove_left(key).unwrap();
        self.nodes.remove(idx.get());
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

    pub fn iter(&self, top: &Key) -> impl Iterator<Item = LayoutItem<Key>> {
        let idx = self.key_to_idx.get_left(top).unwrap();
        LayoutIter::new(self, *idx).map(move |idx| LayoutItem::new(&self, idx))
    }

    pub fn iter_with_obj(&self, top: &Key) -> impl Iterator<Item = LayoutItemWithObj<T>> {
        let idx = self.key_to_idx.get_left(top).unwrap();
        LayoutIter::new(self, *idx).map(move |idx| LayoutItemWithObj::new(&self, idx))
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

    pub fn get_layout(&self, key: &Key) -> Option<(Offset, Size, &T)> {
        self.key_to_idx.get_left(key).map(|idx| {
            let node = &self.nodes[idx];
            (node.abs_pos.get().unwrap(), node.size.get().unwrap(), &node.obj)
        })
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
            any_dirty_children: Cell::new(false),
            input_constraint: Cell::new(None),
            size: Cell::new(None),
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
    fn obj(&self) -> &dyn Layout;

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

    pub fn layout(&self, constraint: BoxConstraints) -> Size {
        // dbg!("before_layout", self.child.obj(), self.child.any_dirty_children(),
        // constraint, self.child.input_constraints(), self.child.size());
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
        // dbg!("after_layout", self.child.obj(), self.child.any_dirty_children(),
        // constraint, self.child.input_constraints(), self.child.size());
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
