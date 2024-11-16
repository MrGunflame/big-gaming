use std::borrow::Borrow;
use std::collections::hash_map;
use std::hash::Hash;
use std::iter::FusedIterator;

use ahash::HashMap;

/// A bidirectional hashmap.
///
/// A `BiMap` bijectively and bidirectionally maps a left key `L` to a right key `R`.
#[derive(Clone, Debug)]
pub struct BiMap<L, R> {
    left: HashMap<L, R>,
    right: HashMap<R, L>,
}

impl<L, R> BiMap<L, R> {
    /// Returns a new, empty `BiMap`.
    pub fn new() -> Self {
        Self {
            left: HashMap::default(),
            right: HashMap::default(),
        }
    }

    /// Returns the number of entries in the `BiMap`.
    pub fn len(&self) -> usize {
        debug_assert_eq!(self.left.len(), self.right.len());
        self.left.len()
    }

    /// Returns `true` if the `BiMap` does not contains any entries.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns an iterator visiting all the entries in the `BiMap`.
    pub fn iter(&self) -> Iter<'_, L, R> {
        Iter {
            inner: self.left.iter(),
        }
    }
}

impl<L, R> BiMap<L, R>
where
    // TODO: Remove the `Copy` bounds.
    L: Hash + Eq + Copy,
    R: Hash + Eq + Copy,
{
    pub fn insert(&mut self, left: L, right: R) {
        self.left.insert(left, right);
        self.right.insert(right, left);
    }

    /// Returns `true` if the `BiMap` contains the given left key.
    pub fn contains_left<Q>(&self, left: &Q) -> bool
    where
        L: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.get_left(left).is_some()
    }

    /// Returns `true` if the `BiMap` contains the given right key.
    pub fn contains_right<Q>(&self, right: &Q) -> bool
    where
        R: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.get_right(right).is_some()
    }

    /// Returns the right key associated with the given left key. Returns `None` if the key does
    /// not exist in the `BiMap`.
    pub fn get_left<Q>(&self, left: &Q) -> Option<&R>
    where
        L: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.left.get(left)
    }

    /// Returns the left key associated with the given right key. Returns `None` if the key does
    /// not exist in the `BiMap`.
    pub fn get_right<Q>(&self, right: &Q) -> Option<&L>
    where
        R: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.right.get(right)
    }

    /// Removes the entry with the given left key and returns the associated right key. Returns
    /// `None` if the key does not exist in the `BiMap`.
    pub fn remove_left<Q>(&mut self, left: &Q) -> Option<R>
    where
        L: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let right = self.left.remove(left)?;
        self.right.remove(&right);
        Some(right)
    }

    /// Removes the entry with the given right key and returns the associated left key. Returns
    /// `None` if the key does not exist in the `BiMap`.
    pub fn remove_right<Q>(&mut self, right: &Q) -> Option<L>
    where
        R: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let left = self.right.remove(right)?;
        self.left.remove(&left);
        Some(left)
    }
}

impl<L, R> Default for BiMap<L, R> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, L, R> IntoIterator for &'a BiMap<L, R> {
    type Item = (&'a L, &'a R);
    type IntoIter = Iter<'a, L, R>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<L, R> IntoIterator for BiMap<L, R> {
    type Item = (L, R);
    type IntoIter = IntoIter<L, R>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter {
            inner: self.left.into_iter(),
        }
    }
}

/// An `Iterator` visiting all entries in a [`BiMap`].
///
/// Returned by [`iter`].
///
/// [`iter`]: BiMap::iter
#[derive(Clone, Debug)]
pub struct Iter<'a, L, R> {
    inner: hash_map::Iter<'a, L, R>,
}

impl<'a, L, R> Iterator for Iter<'a, L, R> {
    type Item = (&'a L, &'a R);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<'a, L, R> ExactSizeIterator for Iter<'a, L, R> {
    fn len(&self) -> usize {
        self.inner.len()
    }
}

impl<'a, L, R> FusedIterator for Iter<'a, L, R> {}

/// A consuming `Iterator` visiting all the entries in a [`BiMap`].
///
/// Returned by [`into_iter`].
///
/// [`into_iter`]: BiMap::into_iter
pub struct IntoIter<L, R> {
    inner: hash_map::IntoIter<L, R>,
}

impl<L, R> Iterator for IntoIter<L, R> {
    type Item = (L, R);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<L, R> ExactSizeIterator for IntoIter<L, R> {
    fn len(&self) -> usize {
        self.inner.len()
    }
}

impl<L, R> FusedIterator for IntoIter<L, R> {}

#[cfg(test)]
mod tests {
    use super::BiMap;

    #[test]
    fn bimap_insert_and_get() {
        let mut map = BiMap::<i32, u32>::new();

        map.insert(-1, 1);
        assert_eq!(map.get_left(&-1), Some(&1));
        assert_eq!(map.get_right(&1), Some(&-1));
    }

    #[test]
    fn bimap_insert_and_remove_left() {
        let mut map = BiMap::<i32, u32>::new();

        map.insert(-1, 1);
        assert_eq!(map.remove_left(&-1), Some(1));
        assert_eq!(map.get_right(&1), None);
        assert!(map.is_empty());
    }

    #[test]
    fn bimap_insert_and_remove_right() {
        let mut map = BiMap::<i32, u32>::new();

        map.insert(-1, 1);
        assert_eq!(map.remove_right(&1), Some(-1));
        assert_eq!(map.get_left(&1), None);
        assert!(map.is_empty());
    }
}
