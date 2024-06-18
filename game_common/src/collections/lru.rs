use std::borrow::Borrow;
use std::hash::Hash;
use std::marker::PhantomPinned;
use std::mem::MaybeUninit;
use std::ptr::NonNull;

use ahash::RandomState;
use hashbrown::HashTable;

use crate::cell::UnsafeRefCell;

pub struct LruCache<K, V> {
    entries: *mut MaybeUninit<Bucket<K, V>>,
    map: HashTable<usize>,
    capacity: usize,
    len: usize,
    head: Option<NonNull<Bucket<K, V>>>,
    tail: Option<NonNull<Bucket<K, V>>>,
    state: RandomState,
}

impl<K, V> LruCache<K, V> {
    pub fn new(capacity: usize) -> Self {
        let mut entries = Vec::with_capacity(capacity);
        entries.resize_with(capacity, || MaybeUninit::uninit());

        let ptr = entries.as_mut_ptr();
        core::mem::forget(entries);

        Self {
            entries: ptr,
            map: HashTable::with_capacity(capacity),
            capacity,
            head: None,
            tail: None,
            len: 0,
            state: RandomState::new(),
        }
    }

    pub fn insert(&mut self, key: K, value: V)
    where
        K: Eq + Hash,
    {
        if self.len == self.capacity {
            self.pop();
        }

        let index = self.len;
        self.len += 1;

        let slot = unsafe { &mut *self.entries.add(index) };
        let bucket = slot.write(Bucket {
            key,
            value,
            pointers: UnsafeRefCell::new(Pointers {
                prev: None,
                next: self.head,
            }),
            _pin: PhantomPinned,
        });

        if let Some(head) = self.head {
            unsafe {
                head.as_ref().pointers.get_mut().prev = Some(bucket.into());
            }
        }

        self.head = Some(bucket.into());
        if self.tail.is_none() {
            self.tail = Some(bucket.into());
        }

        let hasher = self.hasher();
        self.map.insert_unique(hasher(&index), index, hasher);
    }

    fn hasher(&self) -> impl Fn(&usize) -> u64
    where
        K: Hash,
    {
        let entries = self.entries;
        let state = self.state.clone();
        move |index: &usize| {
            let key = unsafe {
                let slot = &*entries.add(*index);
                &slot.assume_init_ref().key
            };
            state.hash_one(key)
        }
    }

    pub fn get<Q>(&mut self, key: Q) -> Option<&V>
    where
        Q: Borrow<K>,
        K: Eq + Hash,
    {
        let hash = self.state.hash_one(key.borrow());
        let index = *self.map.find(hash, |index| unsafe {
            let slot = &*self.entries.add(*index);
            &slot.assume_init_ref().key == key.borrow()
        })?;

        let bucket = unsafe { (&*self.entries.add(index)).assume_init_ref() };

        // Promote the bucket by placing it at `self.head`.
        unsafe {
            let mut pointers = bucket.pointers.get_mut();

            match pointers.next {
                Some(next) => next.as_ref().pointers.get_mut().prev = pointers.prev,
                None => self.tail = pointers.prev,
            }

            match pointers.prev {
                Some(prev) => prev.as_ref().pointers.get_mut().next = pointers.next,
                None => self.head = pointers.next,
            }

            pointers.prev = None;
            pointers.next = self.head;
            self.head = Some(bucket.into());
        }

        Some(unsafe { &(&*self.entries.add(index)).assume_init_ref().value })
    }

    pub fn pop(&mut self) -> Option<(K, V)>
    where
        K: Eq + Hash,
    {
        let tail = self.tail?;

        unsafe {
            let bucket = tail.as_ref();
            let pointers = bucket.pointers.get_mut();

            debug_assert!(pointers.next.is_none());

            match pointers.prev {
                Some(prev) => prev.as_ref().pointers.get_mut().next = None,
                None => self.head = None,
            }

            self.tail = pointers.prev;
        }
    }
}

struct Bucket<K, V> {
    pointers: UnsafeRefCell<Pointers<K, V>>,
    key: K,
    value: V,
    _pin: PhantomPinned,
}

struct Pointers<K, V> {
    prev: Option<NonNull<Bucket<K, V>>>,
    next: Option<NonNull<Bucket<K, V>>>,
}

#[cfg(test)]
mod tests {
    use super::LruCache;

    #[test]
    fn lru_cache() {
        let mut cache = LruCache::new(3);
        cache.insert(0, 0);
        cache.insert(1, 1);
        cache.insert(2, 2);

        assert_eq!(cache.get(2), Some(&2));
        assert_eq!(cache.get(1), Some(&1));
        assert_eq!(cache.get(0), Some(&0));

        cache.insert(3, 3);
        assert_eq!(cache.get(3), Some(&3));
    }
}
