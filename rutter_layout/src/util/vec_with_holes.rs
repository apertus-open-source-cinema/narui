use crate::util::Idx;

#[derive(Debug)]
pub(crate) struct VecWithHoles<T> {
    data: Vec<T>,
    free_list: Vec<usize>,
}

impl<T> VecWithHoles<T> {
    pub(crate) fn new() -> Self { Self { data: Vec::new(), free_list: Vec::new() } }

    pub(crate) fn add(&mut self, value: T) -> usize {
        if let Some(idx) = self.free_list.pop() {
            self.data[idx] = value;
            idx
        } else {
            self.data.push(value);
            self.data.len() - 1
        }
    }

    pub(crate) fn remove(&mut self, idx: usize) { self.free_list.push(idx) }

    // Iters every item even the deleted ones
    pub(crate) fn iter_raw(&self) -> impl Iterator<Item = &T> { self.data.iter() }
}

impl<T> std::ops::Index<usize> for VecWithHoles<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output { &self.data[index] }
}

impl<T> std::ops::IndexMut<usize> for VecWithHoles<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output { &mut self.data[index] }
}

impl<T> std::ops::Index<Idx> for VecWithHoles<T> {
    type Output = T;

    fn index(&self, index: Idx) -> &Self::Output { &self.data[index.get()] }
}

impl<T> std::ops::IndexMut<Idx> for VecWithHoles<T> {
    fn index_mut(&mut self, index: Idx) -> &mut Self::Output { &mut self.data[index.get()] }
}

impl<T> std::ops::Index<&Idx> for VecWithHoles<T> {
    type Output = T;

    fn index(&self, index: &Idx) -> &Self::Output { &self.data[index.get()] }
}

impl<T> std::ops::IndexMut<&Idx> for VecWithHoles<T> {
    fn index_mut(&mut self, index: &Idx) -> &mut Self::Output { &mut self.data[index.get()] }
}
