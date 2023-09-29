use std::hint::unreachable_unchecked;
use std::iter::FusedIterator;
use std::num::NonZeroU32;

#[derive(Clone, Debug)]
pub struct Arena<T> {
    entries: Vec<Entry<T>>,
    len: usize,
    free_head: Option<usize>,
}

impl<T> Arena<T> {
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
            len: 0,
            free_head: None,
        }
    }

    pub fn insert(&mut self, value: T) -> Key {
        self.len += 1;
        assert!(self.len <= u32::MAX as usize);

        if let Some(index) = self.free_head {
            let slot = self.entries.get_mut(index).unwrap();

            let entry = match slot {
                Entry::Occupied(_) => unreachable!(),
                Entry::Vacant(entry) => entry,
            };

            self.free_head = entry.next_free;
            let generation = entry.generation.next();
            *slot = Entry::Occupied(OccupiedEntry { value, generation });

            Key {
                index: index as u32,
                generation,
            }
        } else {
            let generation = Generation::new();
            let index: u32 = self.entries.len().try_into().unwrap();

            self.entries
                .push(Entry::Occupied(OccupiedEntry { value, generation }));

            Key { index, generation }
        }
    }

    pub fn remove(&mut self, key: Key) -> Option<T> {
        let slot = self.entries.get_mut(key.index as usize)?;

        match slot {
            Entry::Occupied(entry) => {
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

    #[inline]
    pub fn contains(&self, key: Key) -> bool {
        match self.entries.get(key.index as usize) {
            Some(Entry::Occupied(entry)) if entry.generation == key.generation => true,
            _ => false,
        }
    }

    #[inline]
    pub fn get(&self, key: Key) -> Option<&T> {
        match self.entries.get(key.index as usize) {
            Some(Entry::Occupied(entry)) if entry.generation == key.generation => {
                Some(&entry.value)
            }
            _ => None,
        }
    }

    #[inline]
    pub fn get_mut(&mut self, key: Key) -> Option<&mut T> {
        match self.entries.get_mut(key.index as usize) {
            Some(Entry::Occupied(entry)) if entry.generation == key.generation => {
                Some(&mut entry.value)
            }
            _ => None,
        }
    }

    #[inline]
    pub unsafe fn get_unchecked(&self, key: Key) -> &T {
        debug_assert!(self.contains(key));

        unsafe {
            match self.entries.get_unchecked(key.index as usize) {
                Entry::Occupied(entry) => &entry.value,
                Entry::Vacant(_) => unreachable_unchecked(),
            }
        }
    }

    #[inline]
    pub unsafe fn get_unchecked_mut(&mut self, key: Key) -> &mut T {
        debug_assert!(self.contains(key));

        unsafe {
            match self.entries.get_unchecked_mut(key.index as usize) {
                Entry::Occupied(entry) => &mut entry.value,
                Entry::Vacant(_) => unreachable_unchecked(),
            }
        }
    }

    pub fn iter(&self) -> Iter<'_, T> {
        Iter {
            arena: self,
            index: 0,
        }
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

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Key {
    index: u32,
    generation: Generation,
}

#[derive(Clone, Debug)]
struct VacantEntry {
    next_free: Option<usize>,
    generation: Generation,
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

                    return Some((key, &entry.value));
                }
                _ => (),
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len(), Some(self.len()))
    }
}

impl<'a, T> ExactSizeIterator for Iter<'a, T> {
    fn len(&self) -> usize {
        self.arena.len as usize
    }
}

impl<'a, T> FusedIterator for Iter<'a, T> {}

#[derive(Debug)]
pub struct IterMut<'a, T> {
    arena: &'a mut Arena<T>,
    index: usize,
}
