use std::borrow::Borrow;
use std::cell::UnsafeCell;
use std::hash::{Hash, Hasher};
use std::hint::unreachable_unchecked;
use std::marker::PhantomData;
use std::num::NonZeroUsize;
use std::ptr::NonNull;

use ahash::{HashMap, HashMapExt, RandomState};
use hashbrown::HashTable;

use crate::cell::UnsafeRefCell;

use super::arena::Key;

/// A least-recently-used cache.
///
/// `LruCache` is fixed-size cache that drops the least recently used entries when its capacity is
/// reached.
// #[derive(Debug)]
pub struct LruCache<K, V> {
    /// Map of key-value pairs.
    ///
    /// We heap allocate every key-value in a [`Bucket`]. The [`KeyPtr`] from a entry points to
    /// the key `K` within the heap-allocated [`Bucket`].
    ///
    /// Therefore we MUST NOT drop the associated [`Bucket`] before removing the pair from the map.
    entries: Box<[Entry<K, V>]>,
    free_head: Option<usize>,
    // TODO: We can maybe make this more performant by reducing it to
    // just two allocated objects. A array stores all the buckets inline and
    // the hashmap collects pointers/indices into the array.
    map: HashTable<usize>,
    /// Pointer to the most recently used entry.
    ///
    /// This is where new entries will be inserted and accessed entries will be promoted to.
    head: Option<usize>,
    /// Pointer to the least recently used entry.
    ///
    /// This is where entries will be evicted from the cache if the capacity is reached.
    tail: Option<usize>,
    hash_builder: RandomState,
}

impl<K, V> LruCache<K, V> {
    pub fn new(capacity: usize) -> Self {
        let mut entries = Vec::with_capacity(capacity);
        for index in 0..capacity {
            let index = if index < capacity {
                Some(index + 1)
            } else {
                None
            };

            entries.push(Entry::Free(index));
        }

        Self {
            entries: entries.into_boxed_slice(),
            free_head: Some(0),
            map: HashTable::with_capacity(capacity),
            head: None,
            tail: None,
            hash_builder: RandomState::new(),
        }
    }

    /// Returns the number of entries in the `LruCache`.
    #[inline]
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Returns `true` if the `LruCache` contains no entries.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the maximum number of entries that can be stored in the `LruCache`.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.map.capacity()
    }

    /// Returns `true` if the `LruCache` is at maximum capacity.
    #[inline]
    pub fn is_full(&self) -> bool {
        self.len() == self.capacity()
    }

    /// Inserts a new entry into the `LruCache`.
    ///
    /// The new entry will be declared as the most recently used entry and evict the least recently
    /// used entry if the `LruCache` is full.
    pub fn insert(&mut self, key: K, value: V)
    where
        K: Eq + Hash,
    {
        if self.is_full() {
            self.pop();
        }

        let hash = self.hash_builder.hash_one(&key);

        let bucket = Bucket {
            value: UnsafeCell::new(value),
            key,
            pointers: UnsafeRefCell::new(Pointers {
                prev: None,
                next: self.head,
            }),
        };

        let index = self.alloc(bucket);

        match self.head {
            Some(head) => unsafe {
                let head = self.entries.get_unchecked_mut(head);
                head.as_bucket_unchecked_mut().pointers.get_mut_safe().prev = Some(index);
            },
            None => self.tail = Some(index),
        }
        self.head = Some(index);

        // match self.map.find_entry(hash, |(v, _)| v == &key) {
        //     Ok(occupied) => {
        //         todo!()
        //     }
        //     Err(vacant) => {}
        // }
        self.map.insert_unique(hash, index, |index| {
            let bucket = unsafe { self.entries.get_unchecked(*index).as_bucket_unchecked() };
            self.hash_builder.hash_one(&bucket.key)
        });
    }

    /// Returns a reference to a value in the `LruCache`.
    ///
    /// If the value for the given `key` exists the entry is promoted to the most recently used
    /// entry.
    pub fn get<Q>(&mut self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q> + Eq + Hash,
        Q: Hash + Eq + ?Sized,
    {
        self.get_mut(key).map(|v| &*v)
    }

    /// Returns a mutable reference to a value in the `LruCache`.
    ///
    /// If the value for the given `key` exists the entry is promoted to the most recently used
    /// entry.
    pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut V>
    where
        K: Borrow<Q> + Hash + Eq,
        Q: Hash + Eq + ?Sized,
    {
        let hash = self.hash_builder.hash_one(key);
        let &index = self.map.find(hash, |index| unsafe {
            self.entries
                .get_unchecked(*index)
                .as_bucket_unchecked()
                .key
                .borrow()
                == key
        })?;

        // let key_ref: &KeyRef<Q> = KeyRef::from_ref(key);
        // let key = *self.map.get(key_ref)?;

        debug_assert!(self.head.is_some());
        debug_assert!(self.tail.is_some());

        // Promote the bucket by placing it at `self.head`.
        unsafe {
            let bucket = self
                .entries
                .get_unchecked_mut(index)
                .as_bucket_unchecked_mut();
            let pointers = bucket.pointers.get_mut_safe().clone();

            if cfg!(debug_assertions) {
                if let (Some(next), Some(prev)) = (pointers.next, pointers.prev) {
                    assert_ne!(index, next);
                    assert_ne!(next, prev);
                }
            }

            // Remove the entry from the linked list.

            match pointers.next {
                Some(next) => {
                    let next = self
                        .entries
                        .get_unchecked_mut(next)
                        .as_bucket_unchecked_mut();
                    next.pointers.get_mut_safe().prev = pointers.prev;
                }
                None => self.tail = pointers.prev,
            }

            match pointers.prev {
                Some(prev) => {
                    let prev = self
                        .entries
                        .get_unchecked_mut(prev)
                        .as_bucket_unchecked_mut();
                    prev.pointers.get_mut_safe().next = pointers.next;
                }
                None => self.head = pointers.next,
            }

            // self.insert_bucket(bucket);
            match self.head {
                Some(head) => {
                    self.entries
                        .get_unchecked_mut(head)
                        .as_bucket_unchecked_mut()
                        .pointers
                        .get_mut_safe()
                        .prev = Some(index)
                }
                None => self.tail = Some(index),
            }

            self.head = Some(index);

            let bucket = self
                .entries
                .get_unchecked_mut(index)
                .as_bucket_unchecked_mut();
            Some(bucket.value.get_mut())

            // Some(&mut *bucket.value.get())
        }
    }

    // fn insert_bucket(&mut self, mut bucket: Bucket<K, V>) -> Key {
    //     bucket.pointers.get_mut_safe().prev = None;
    //     bucket.pointers.get_mut_safe().next = self.head;

    //     let key = self.entries.insert(bucket);

    //     match self.head {
    //         Some(head) => unsafe {
    //             self.entries
    //                 .get_unchecked_mut(head)
    //                 .pointers
    //                 .get_mut_safe()
    //                 .prev = Some(key)
    //         },
    //         None => self.tail = Some(key),
    //     }

    //     self.head = Some(key);
    //     key
    // }

    /// Removes the least recently used entry from the `LruCache`.
    pub fn pop(&mut self) -> Option<(K, V)>
    where
        K: Eq + Hash,
    {
        let tail = self.tail?;

        let bucket = unsafe { self.entries.get_unchecked(tail).as_bucket_unchecked() };

        let hash = self.hash_builder.hash_one(&bucket.key);
        match self.map.find_entry(hash, |index| unsafe {
            self.entries.get_unchecked(*index).as_bucket_unchecked().key == bucket.key
        }) {
            Ok(entry) => {
                debug_assert_eq!(*entry.get(), tail);
                entry.remove();
            }
            Err(_) => unsafe { unreachable_unchecked() },
        }

        // let res = self
        //     .map
        //     .remove(&KeyPtr::from_bucket(tail.as_ptr().cast_const()));
        // debug_assert_eq!(res, Some(tail));

        unsafe {
            // let boxed = Box::from_raw(tail.as_ptr());
            // let pointers = boxed.pointers.get_mut();
            let pointers = bucket.pointers.get().clone();

            match pointers.prev {
                Some(prev) => {
                    let prev = self
                        .entries
                        .get_unchecked_mut(prev)
                        .as_bucket_unchecked_mut();
                    prev.pointers.get_mut_safe().next = None
                }
                None => self.head = None,
            }

            self.tail = pointers.prev;

            // let bucket = self.entries.remove(tail).unwrap();
            let bucket = self.dealloc(tail);

            // Some((boxed.key, boxed.value.into_inner()))
            Some((bucket.key, bucket.value.into_inner()))
        }
    }

    fn alloc(&mut self, bucket: Bucket<K, V>) -> usize {
        match self.free_head {
            Some(index) => unsafe {
                let slot = self.entries.get_unchecked_mut(index);

                let next_free = match slot {
                    Entry::Bucket(_) => unreachable_unchecked(),
                    Entry::Free(next_free) => *next_free,
                };

                self.free_head = next_free;
                *slot = Entry::Bucket(bucket);
                index
            },
            None => todo!(),
        }
    }

    unsafe fn dealloc(&mut self, index: usize) -> Bucket<K, V> {
        unsafe {
            let slot = self.entries.get_unchecked_mut(index);

            let new = Entry::Free(self.free_head);

            self.free_head = Some(index);

            match core::mem::replace(slot, new) {
                Entry::Bucket(bucket) => bucket,
                Entry::Free(_) => unreachable_unchecked(),
            }
        }
    }
}

impl<K, V> Drop for LruCache<K, V> {
    fn drop(&mut self) {
        // for (_, bucket) in self.map.drain() {
        //     unsafe {
        //         drop(Box::from_raw(bucket.as_ptr()));
        //     }
        // }
    }
}

unsafe impl<K, V> Sync for LruCache<K, V>
where
    K: Sync,
    V: Sync,
{
}

unsafe impl<K, V> Send for LruCache<K, V>
where
    K: Send,
    V: Send,
{
}

#[derive(Debug)]
struct KeyPtr<K> {
    ptr: *const K,
}

impl<K> KeyPtr<K> {
    fn from_bucket<V>(bucket: *const Bucket<K, V>) -> Self {
        let offset = core::mem::offset_of!(Bucket<K, V>, key);

        Self {
            ptr: unsafe { bucket.byte_add(offset).cast::<K>() },
        }
    }

    fn as_ref(&self) -> &K {
        unsafe { &*self.ptr }
    }
}

impl<K> Hash for KeyPtr<K>
where
    K: Hash,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_ref().hash(state);
    }
}

impl<K> PartialEq for KeyPtr<K>
where
    K: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.as_ref().eq(other.as_ref())
    }
}

impl<K> Eq for KeyPtr<K> where K: Eq {}

#[repr(transparent)]
struct KeyRef<K>(K)
where
    K: ?Sized;

impl<K> KeyRef<K>
where
    K: ?Sized,
{
    fn from_ref(key: &K) -> &Self {
        unsafe { core::mem::transmute(key) }
    }
}

impl<K> Hash for KeyRef<K>
where
    K: ?Sized + Hash,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl<K> PartialEq for KeyRef<K>
where
    K: ?Sized + PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}

impl<K> Eq for KeyRef<K> where K: ?Sized + Eq {}

impl<K, Q> Borrow<KeyRef<Q>> for KeyPtr<K>
where
    K: Borrow<Q>,
    Q: ?Sized,
{
    fn borrow(&self) -> &KeyRef<Q> {
        KeyRef::from_ref(self.as_ref().borrow())
    }
}

struct Bucket<K, V> {
    pointers: UnsafeRefCell<Pointers>,
    key: K,
    // We need to wrap `value` in a `UnsafeCell` to allow borrowing it
    // mutably without having to borrow the entire `Bucket`, which
    // would cause UB since an immutable `KeyRef` to the `Bucket` may
    // exist.
    value: UnsafeCell<V>,
}

#[derive(Clone, Debug)]
struct Pointers {
    prev: Option<usize>,
    next: Option<usize>,
}

enum Entry<K, V> {
    Bucket(Bucket<K, V>),
    Free(Option<usize>),
}

impl<K, V> Entry<K, V> {
    unsafe fn as_bucket_unchecked(&self) -> &Bucket<K, V> {
        match self {
            Self::Bucket(bucket) => bucket,
            Self::Free(_) => unsafe { unreachable_unchecked() },
        }
    }

    unsafe fn as_bucket_unchecked_mut(&mut self) -> &mut Bucket<K, V> {
        match self {
            Self::Bucket(bucket) => bucket,
            Self::Free(_) => unsafe { unreachable_unchecked() },
        }
    }
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

        assert_eq!(cache.get(&2), Some(&2));
        assert_eq!(cache.get(&1), Some(&1));
        assert_eq!(cache.get(&0), Some(&0));

        cache.insert(3, 3);
        assert_eq!(cache.get(&3), Some(&3));
    }

    #[test]
    fn lru_cache_insert_with_overflow() {
        let mut cache = LruCache::new(3);
        cache.insert(0, 0);
        cache.insert(1, 1);
        cache.insert(2, 2);
        cache.insert(3, 3);

        assert_eq!(cache.get(&0), None);
        assert_eq!(cache.get(&1), Some(&1));
        assert_eq!(cache.get(&2), Some(&2));
        assert_eq!(cache.get(&3), Some(&3));
    }

    #[test]
    fn lru_cache_pop() {
        let mut cache = LruCache::new(3);
        cache.insert(0, 0);
        cache.insert(1, 1);
        cache.insert(2, 2);

        assert_eq!(cache.pop(), Some((0, 0)));
        assert_eq!(cache.pop(), Some((1, 1)));
        assert_eq!(cache.pop(), Some((2, 2)));
        assert_eq!(cache.pop(), None);
    }
}
