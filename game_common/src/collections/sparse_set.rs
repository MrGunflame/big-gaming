use std::iter::FusedIterator;

#[derive(Clone, Debug)]
pub struct SparseSet<T> {
    sparse: Vec<usize>,
    dense: Vec<T>,
}

impl<T> SparseSet<T> {
    pub const fn new() -> Self {
        Self {
            sparse: Vec::new(),
            dense: Vec::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            sparse: Vec::with_capacity(capacity),
            dense: Vec::with_capacity(capacity),
        }
    }

    pub fn len(&self) -> usize {
        self.dense.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get(&self, key: usize) -> Option<&T> {
        let index = *self.sparse.get(key)?;
        let value = self.dense.get(index).unwrap();
        Some(value)
    }

    pub fn get_mut(&mut self, key: usize) -> Option<&mut T> {
        let index = *self.sparse.get(key)?;
        let value = self.dense.get_mut(index).unwrap();
        Some(value)
    }

    pub fn remove(&mut self, key: usize) -> Option<T> {
        let index = *self.sparse.get(key)?;
        let value = self.dense.swap_remove(index);
        Some(value)
    }

    pub fn insert(&mut self, value: T) -> usize {
        let dense_index = self.dense.len();
        self.dense.push(value);

        let index = self.sparse.len();
        self.sparse.push(dense_index);

        index
    }

    pub fn iter(&self) -> Iter<'_, T> {
        Iter {
            dense: self.dense.iter(),
        }
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        IterMut {
            dense: self.dense.iter_mut(),
        }
    }
}

impl<'a, T> IntoIterator for &'a SparseSet<T> {
    type Item = <Self::IntoIter as Iterator>::Item;
    type IntoIter = Iter<'a, T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut SparseSet<T> {
    type Item = <Self::IntoIter as Iterator>::Item;
    type IntoIter = IterMut<'a, T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

#[derive(Clone, Debug)]
pub struct Iter<'a, T> {
    dense: std::slice::Iter<'a, T>,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.dense.next()
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.dense.size_hint()
    }
}

impl<'a, T> ExactSizeIterator for Iter<'a, T> {
    #[inline]
    fn len(&self) -> usize {
        self.dense.len()
    }
}

impl<'a, T> FusedIterator for Iter<'a, T> {}

#[derive(Debug)]
pub struct IterMut<'a, T> {
    dense: std::slice::IterMut<'a, T>,
}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = &'a mut T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.dense.next()
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.dense.size_hint()
    }
}

impl<'a, T> ExactSizeIterator for IterMut<'a, T> {
    #[inline]
    fn len(&self) -> usize {
        self.dense.len()
    }
}

impl<'a, T> FusedIterator for IterMut<'a, T> {}

#[cfg(test)]
mod tests {
    use super::SparseSet;

    #[test]
    fn insert_get() {
        let mut set = SparseSet::new();

        for index in 0..128 {
            assert_eq!(set.len(), index);

            let key = set.insert(index);
            assert_eq!(*set.get(key).unwrap(), index);
        }
    }

    #[test]
    fn insert_get_remove() {
        let mut set = SparseSet::new();

        for index in 0..128 {
            let key = set.insert(index);
            assert_eq!(*set.get(key).unwrap(), index);
            assert_eq!(set.remove(key).unwrap(), index);
        }

        assert_eq!(set.len(), 0);
    }
}
