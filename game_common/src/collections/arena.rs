use std::hint::unreachable_unchecked;
use std::iter::FusedIterator;
use std::num::NonZeroU32;

/// A generic container for values with fast insertion, access and removal.
#[derive(Clone, Debug)]
pub struct Arena<T> {
    entries: Vec<Entry<T>>,
    len: usize,
    free_head: Option<usize>,
}

impl<T> Arena<T> {
    /// Creates a new, empty `Arena`.
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
            len: 0,
            free_head: None,
        }
    }

    /// Creates a new `Arena` preallocated with the specified `capacity`.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            entries: Vec::with_capacity(capacity),
            len: 0,
            free_head: None,
        }
    }

    /// Returns the number of elements in the `Arena`.
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Returns `true` if the `Arena` contains no elements.
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Inserts a new value into the `Arena`.
    ///
    /// # Panics
    ///
    /// Panics if the `Arena` is at maximum capacity.
    pub fn insert(&mut self, value: T) -> Key {
        #[inline(never)]
        #[cold]
        fn panic_on_err() -> ! {
            panic!("`Arena` is at maximum capacity")
        }

        match self.try_insert(value) {
            Ok(key) => key,
            Err(_) => panic_on_err(),
        }
    }

    /// Attempts to inserts a new value into the `Arena`, returning an `Err` if the `Arena` is at
    /// maximum capacity.
    pub fn try_insert(&mut self, value: T) -> Result<Key, T> {
        // Attempt to increment length.
        match self.len.checked_add(1) {
            Some(len) => self.len = len,
            None => return Err(value),
        }

        if let Some(index) = self.free_head {
            let slot = self.entries.get_mut(index).unwrap();

            let entry = match slot {
                Entry::Occupied(_) => unreachable!(),
                Entry::Vacant(entry) => entry,
            };

            self.free_head = entry.next_free;
            let generation = entry.generation.next();
            *slot = Entry::Occupied(OccupiedEntry { value, generation });

            Ok(Key {
                index: index as u32,
                generation,
            })
        } else {
            let generation = Generation::new();
            let index: u32 = self.entries.len().try_into().unwrap();

            self.entries
                .push(Entry::Occupied(OccupiedEntry { value, generation }));

            Ok(Key { index, generation })
        }
    }

    /// Removes and returns an element from the `Arena`.
    pub fn remove(&mut self, key: Key) -> Option<T> {
        let slot = self.entries.get_mut(key.index as usize)?;

        match slot {
            Entry::Occupied(entry) => {
                self.len -= 1;

                let new = Entry::Vacant(VacantEntry {
                    next_free: self.free_head,
                    generation: entry.generation,
                });

                let value = std::mem::replace(slot, new);
                Some(match value {
                    Entry::Occupied(e) => e.value,
                    _ => unreachable!(),
                })
            }
            Entry::Vacant(_) => None,
        }
    }

    /// Returns `true` if the element with the `key` exists in the `Arena`.
    #[inline]
    pub fn contains_key(&self, key: Key) -> bool {
        match self.entries.get(key.index as usize) {
            Some(Entry::Occupied(entry)) if entry.generation == key.generation => true,
            _ => false,
        }
    }

    /// Returns a reference to the elment with the given `key`. Returns `None` if the `key` does
    /// not exist in the `Arena`.
    #[inline]
    pub fn get(&self, key: Key) -> Option<&T> {
        match self.entries.get(key.index as usize) {
            Some(Entry::Occupied(entry)) if entry.generation == key.generation => {
                Some(&entry.value)
            }
            _ => None,
        }
    }

    /// Returns a mutable reference to the element with the given `key`. Returns `None` if the
    /// `key` does not exist in the `Arena`.
    #[inline]
    pub fn get_mut(&mut self, key: Key) -> Option<&mut T> {
        match self.entries.get_mut(key.index as usize) {
            Some(Entry::Occupied(entry)) if entry.generation == key.generation => {
                Some(&mut entry.value)
            }
            _ => None,
        }
    }

    /// Returns a reference to the element with the given `key`.
    ///
    /// # Safety
    ///
    /// The given `key` must exist within the `Arena`.
    #[inline]
    pub unsafe fn get_unchecked(&self, key: Key) -> &T {
        debug_assert!(self.contains_key(key));

        unsafe {
            match self.entries.get_unchecked(key.index as usize) {
                Entry::Occupied(entry) => &entry.value,
                Entry::Vacant(_) => unreachable_unchecked(),
            }
        }
    }

    /// Returns a mutable reference to the element with the given `key`.
    ///
    /// # Safety
    ///
    /// The given `key` must exist within the `Arena`.
    #[inline]
    pub unsafe fn get_unchecked_mut(&mut self, key: Key) -> &mut T {
        debug_assert!(self.contains_key(key));

        unsafe {
            match self.entries.get_unchecked_mut(key.index as usize) {
                Entry::Occupied(entry) => &mut entry.value,
                Entry::Vacant(_) => unreachable_unchecked(),
            }
        }
    }

    /// Returns an `Iterator` over the elements within the `Arena`.
    pub fn iter(&self) -> Iter<'_, T> {
        Iter {
            arena: self,
            index: 0,
            len: self.len,
        }
    }

    /// Returns an `Iterator` over the elements within the `Arena`.
    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        IterMut {
            iter: self.entries.iter_mut(),
            index: 0,
            len: self.len,
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.len = 0;
        self.free_head = None;
    }

    pub fn values(&self) -> Values<'_, T> {
        Values { iter: self.iter() }
    }
}

impl<'a, T> IntoIterator for &'a Arena<T> {
    type Item = <Self::IntoIter as Iterator>::Item;
    type IntoIter = Iter<'a, T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut Arena<T> {
    type Item = <Self::IntoIter as Iterator>::Item;
    type IntoIter = IterMut<'a, T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

#[derive(Clone, Debug)]
enum Entry<T> {
    Occupied(OccupiedEntry<T>),
    Vacant(VacantEntry),
}

#[derive(Clone, Debug)]
struct OccupiedEntry<T> {
    value: T,
    generation: Generation,
}

#[derive(Clone, Debug)]
struct VacantEntry {
    next_free: Option<usize>,
    generation: Generation,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Key {
    index: u32,
    generation: Generation,
}

impl Key {
    pub const DANGLING: Self = Self {
        index: u32::MAX,
        generation: Generation(NonZeroU32::MAX),
    };

    #[inline]
    pub const fn index(&self) -> usize {
        self.index as usize
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
struct Generation(NonZeroU32);

impl Generation {
    fn new() -> Self {
        Self(unsafe { NonZeroU32::new_unchecked(1) })
    }

    fn next(self) -> Self {
        Self(self.0.checked_add(1).unwrap_or(Self::new().0))
    }
}

#[derive(Clone, Debug)]
pub struct Iter<'a, T> {
    arena: &'a Arena<T>,
    index: usize,
    len: usize,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = (Key, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let slot = self.arena.entries.get(self.index)?;

            match slot {
                Entry::Occupied(entry) => {
                    let key = Key {
                        index: self.index as u32,
                        generation: entry.generation,
                    };

                    self.index += 1;
                    self.len -= 1;

                    return Some((key, &entry.value));
                }
                Entry::Vacant(_) => {
                    self.index += 1;
                }
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

impl<'a, T> ExactSizeIterator for Iter<'a, T> {
    fn len(&self) -> usize {
        self.len
    }
}

impl<'a, T> FusedIterator for Iter<'a, T> {}

#[derive(Debug)]
pub struct IterMut<'a, T> {
    iter: std::slice::IterMut<'a, Entry<T>>,
    index: usize,
    len: usize,
}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = (Key, &'a mut T);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.iter.next() {
                Some(entry) => match entry {
                    Entry::Occupied(entry) => {
                        let key = Key {
                            index: self.index as u32,
                            generation: entry.generation,
                        };
                        self.index += 1;

                        return Some((key, &mut entry.value));
                    }
                    Entry::Vacant(_) => {
                        self.index += 1;
                    }
                },
                None => return None,
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

impl<'a, T> ExactSizeIterator for IterMut<'a, T> {
    fn len(&self) -> usize {
        self.len
    }
}

impl<'a, T> FusedIterator for IterMut<'a, T> {}

#[derive(Clone, Debug)]
pub struct Values<'a, T> {
    iter: Iter<'a, T>,
}

impl<'a, T> Iterator for Values<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(_, v)| v)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<'a, T> ExactSizeIterator for Values<'a, T> {
    fn len(&self) -> usize {
        self.iter.len()
    }
}

impl<'a, T> FusedIterator for Values<'a, T> {}

#[cfg(test)]
mod tests {
    use super::Arena;

    #[test]
    fn insert_get() {
        let mut arena = Arena::new();

        for index in 0..128 {
            assert_eq!(arena.len(), index);

            let key = arena.insert(index);
            assert_eq!(*arena.get(key).unwrap(), index);
        }
    }

    #[test]
    fn insert_get_remove() {
        let mut arena = Arena::new();

        for index in 0..128 {
            let key = arena.insert(index);
            assert_eq!(*arena.get(key).unwrap(), index);
            assert_eq!(arena.remove(key).unwrap(), index);
        }

        assert_eq!(arena.len(), 0);
    }
}
