use std::iter::FusedIterator;

pub trait Index: Copy + Eq {
    fn index(&self) -> usize;
}

impl Index for super::arena::Key {
    fn index(&self) -> usize {
        self.index()
    }
}

impl Index for usize {
    fn index(&self) -> usize {
        *self
    }
}

/// A key-value map backed by a [`Vec`].
///
/// This is more efficient alternative to a [`HashMap`] when the keys follow a linear pattern.
#[derive(Clone, Debug)]
pub struct VecMap<K, V> {
    inner: Vec<Entry<K, V>>,
    len: usize,
}

impl<K, V> VecMap<K, V> {
    pub const fn new() -> Self {
        Self {
            inner: Vec::new(),
            len: 0,
        }
    }
}

impl<K, V> VecMap<K, V>
where
    K: Index,
{
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        self.inner.resize_with(key.index() + 1, || Entry::None);

        let slot = &mut self.inner[key.index()];
        let old_value = std::mem::replace(slot, Entry::Occupied((key, value)));

        match old_value {
            Entry::None => {
                self.len += 1;
                None
            }
            Entry::Occupied((_, val)) => Some(val),
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get(&self, key: K) -> Option<&V> {
        let entry = self.inner.get(key.index())?;
        match entry {
            Entry::Occupied((k, v)) if *k == key => Some(v),
            _ => None,
        }
    }

    pub fn get_mut(&mut self, key: K) -> Option<&mut V> {
        let entry = self.inner.get_mut(key.index())?;
        match entry {
            Entry::Occupied((k, v)) if *k == key => Some(v),
            _ => None,
        }
    }

    pub fn remove(&mut self, key: K) -> Option<V> {
        let entry = self.inner.get_mut(key.index())?;
        match entry {
            Entry::Occupied((k, _)) if *k == key => {
                let entry = std::mem::replace(entry, Entry::None);
                self.len -= 1;
                match entry {
                    Entry::Occupied((_, v)) => Some(v),
                    _ => unreachable!(),
                }
            }
            _ => None,
        }
    }

    pub fn iter(&self) -> Iter<'_, K, V> {
        Iter {
            iter: self.inner.iter(),
            len: self.len,
        }
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, K, V> {
        IterMut {
            iter: self.inner.iter_mut(),
            len: self.len,
        }
    }

    pub fn values_mut(&mut self) -> ValuesMut<'_, K, V> {
        ValuesMut {
            iter: self.iter_mut(),
        }
    }

    pub fn clear(&mut self) {
        self.inner.clear();
        self.len = 0;
    }
}

impl<K, V> Default for VecMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug)]
enum Entry<K, V> {
    None,
    Occupied((K, V)),
}

pub struct Iter<'a, K, V> {
    iter: std::slice::Iter<'a, Entry<K, V>>,
    len: usize,
}

impl<'a, K, V> Iterator for Iter<'a, K, V>
where
    K: Index,
{
    type Item = (K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let entry = self.iter.next()?;
            match entry {
                Entry::None => (),
                Entry::Occupied((key, val)) => {
                    self.len -= 1;
                    return Some((*key, val));
                }
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

impl<'a, K, V> ExactSizeIterator for Iter<'a, K, V>
where
    K: Index,
{
    fn len(&self) -> usize {
        self.len
    }
}

#[derive(Debug)]
pub struct IterMut<'a, K, V> {
    iter: std::slice::IterMut<'a, Entry<K, V>>,
    len: usize,
}

impl<'a, K, V> Iterator for IterMut<'a, K, V>
where
    K: Index,
{
    type Item = (K, &'a mut V);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let entry = self.iter.next()?;
            match entry {
                Entry::None => (),
                Entry::Occupied((key, val)) => {
                    self.len -= 1;
                    return Some((*key, val));
                }
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

impl<'a, K, V> ExactSizeIterator for IterMut<'a, K, V>
where
    K: Index,
{
    fn len(&self) -> usize {
        self.len
    }
}

impl<'a, K, V> FusedIterator for IterMut<'a, K, V> where K: Index {}

#[derive(Debug)]
pub struct ValuesMut<'a, K, V>
where
    K: Index,
{
    iter: IterMut<'a, K, V>,
}

impl<'a, K, V> Iterator for ValuesMut<'a, K, V>
where
    K: Index,
{
    type Item = &'a mut V;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(_, v)| v)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<'a, K, V> ExactSizeIterator for ValuesMut<'a, K, V>
where
    K: Index,
{
    fn len(&self) -> usize {
        self.iter.len()
    }
}

impl<'a, K, V> FusedIterator for ValuesMut<'a, K, V> where K: Index {}
