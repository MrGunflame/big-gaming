use std::collections::HashMap;

use game_common::cell::{RefMut, UnsafeRefCell};

/// A collection that can be used by both a [`Viewer`] and a [`Writer`] at the same time.
///
/// This is meant to be used as a "transfer channel" between two threads:
/// - One side inserts and removes values. This is meant to be the game logic side.
/// - One side reads and modifies the value. This is meant to be the thread uploading data to the
/// GPU over a transfer queue.
///
///
/// The uploading thread has the ability to "move" values from CPU to GPU memory. This is why
/// the [`Writer`] does not have access to it after insertion.
///
/// A `Pool` requires explicit synchronization between both threads. This is done by calling
/// [`commit`], which commits all insertions/removals into the `Pool`.
///
/// [`commit`]: Self::commit
// FIXME: This probably is not the best name for this type.
#[derive(Debug)]
pub(crate) struct Pool<T> {
    values: UnsafeRefCell<HashMap<usize, T>>,
    writer: UnsafeRefCell<WriterState<T>>,
}

#[derive(Clone, Debug, Default)]
struct WriterState<T> {
    free: Vec<usize>,
    next: usize,
    queued: HashMap<usize, T>,
    queued_deletion: Vec<usize>,
}

impl<T> Pool<T> {
    /// Creates a new, empty `Pool`.
    pub(crate) fn new() -> Self {
        Self {
            values: UnsafeRefCell::new(HashMap::new()),
            writer: UnsafeRefCell::new(WriterState {
                free: Vec::new(),
                next: 0,
                queued: HashMap::new(),
                queued_deletion: Vec::new(),
            }),
        }
    }

    /// Creates a new [`Writer`] bound to this `Pool`.
    ///
    /// # Safety
    ///
    /// At most one [`Writer`] may exist for one `Pool` at the same time.
    pub(crate) unsafe fn writer(&self) -> Writer<'_, T> {
        Writer { pool: self }
    }

    /// Creates a new [`Viewer`] bound to this `Pool`.
    ///
    /// # Safety
    ///
    /// At most one [`Viewer`] may exist for one `Pool` at the same time.
    pub unsafe fn viewer(&self) -> Viewer<'_, T> {
        Viewer {
            values: unsafe { self.values.borrow_mut() },
        }
    }

    /// Commits all staged operations.
    ///
    /// # Safety
    ///
    /// No [`Viewer`]s and [`Writer`]s must exist when calling this function.
    pub unsafe fn commit(&self) {
        let mut values = unsafe { self.values.borrow_mut() };
        let state = unsafe { &mut *self.writer.borrow_mut() };

        for (key, value) in state.queued.drain() {
            debug_assert!(!values.contains_key(&key));
            values.insert(key, value);
        }

        for key in state.queued_deletion.drain(..) {
            // If the slot was cleared it can be reused,
            // but we must not mark the slot as free if a key
            // that doesn't exist was requested for deletion.
            if values.remove(&key).is_some() {
                state.free.push(key);
            }
        }
    }
}

impl<T> Default for Pool<T> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub(crate) struct Viewer<'a, T> {
    values: RefMut<'a, HashMap<usize, T>>,
}

impl<'a, T> Viewer<'a, T> {
    pub(crate) fn get(&self, key: usize) -> Option<&T> {
        self.values.get(&key)
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &T> {
        self.values.values()
    }

    pub(crate) fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.values.values_mut()
    }
}

#[derive(Debug)]
pub(crate) struct Writer<'a, T> {
    pool: &'a Pool<T>,
}

impl<'a, T> Writer<'a, T> {
    pub(crate) fn insert(&mut self, value: T) -> usize {
        let mut state = unsafe { self.pool.writer.borrow_mut() };
        let key = match state.free.pop() {
            Some(key) => key,
            None => {
                let key = state.next;
                state.next += 1;
                key
            }
        };

        state.queued.insert(key, value);
        key
    }

    pub(crate) fn remove(&mut self, key: usize) {
        let mut state = unsafe { self.pool.writer.borrow_mut() };

        if state.queued.remove(&key).is_some() {
            return;
        }

        state.queued_deletion.push(key);
    }
}

#[cfg(test)]
mod tests {
    use super::{Pool, Viewer, Writer};

    fn split_pool<T>(pool: &mut Pool<T>) -> (Writer<'_, T>, Viewer<'_, T>) {
        // SAFETY: Since we have exclusive ownership of the pool for
        // the lifetime of the returned writer/viewer we can guarantee
        // that no other writers/viewers can be created using the same pool.
        unsafe { (pool.writer(), pool.viewer()) }
    }

    #[test]
    fn pool_insert() {
        let mut pool = Pool::<i32>::new();
        let (mut writer, viewer) = split_pool(&mut pool);

        let id0 = writer.insert(0);
        let id1 = writer.insert(1);

        // New state not yet commited.
        assert_eq!(viewer.get(id0), None);
        assert_eq!(viewer.get(id1), None);

        unsafe {
            drop((writer, viewer));
            pool.commit();
        }

        let (_, viewer) = split_pool(&mut pool);

        // New state committed now.
        assert_eq!(viewer.get(id0), Some(&0));
        assert_eq!(viewer.get(id1), Some(&1));
    }

    #[test]
    fn pool_remove() {
        let mut pool = Pool::<i32>::new();
        let (mut writer, viewer) = split_pool(&mut pool);

        let id0 = writer.insert(0);
        let id1 = writer.insert(1);

        unsafe {
            drop((writer, viewer));
            pool.commit();
        }

        let (mut writer, viewer) = split_pool(&mut pool);

        writer.remove(id0);
        writer.remove(id1);

        assert_eq!(viewer.get(id0), Some(&0));
        assert_eq!(viewer.get(id1), Some(&1));

        unsafe {
            drop((writer, viewer));
            pool.commit();
        }

        let (_, viewer) = split_pool(&mut pool);

        assert_eq!(viewer.get(id0), None);
        assert_eq!(viewer.get(id1), None);
    }
}
