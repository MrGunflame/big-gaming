use std::borrow::Borrow;
use std::hash::{Hash, Hasher};
use std::ptr::NonNull;

use ahash::{HashMap, HashMapExt};

use crate::cell::UnsafeRefCell;

/// A least-recently-used cache.
///
/// `LruCache` is fixed-size cache that drops the least recently used entries when its capacity is
/// reached.
#[derive(Debug)]
pub struct LruCache<K, V> {
    /// Map of key-value pairs.
    ///
    /// We heap allocate every key-value in a [`Bucket`]. The [`KeyPtr`] from a entry points to
    /// the key `K` within the heap-allocated [`Bucket`].
    ///
    /// Therefore we MUST NOT drop the associated [`Bucket`] before removing the pair from the map.
    // TODO: We can maybe make this more performant by reducing it to
    // just two allocated objects. A array stores all the buckets inline and
    // the hashmap collects pointers/indices into the array.
    map: HashMap<KeyPtr<K>, NonNull<Bucket<K, V>>>,
    /// Pointer to the most recently used entry.
    ///
    /// This is where new entries will be inserted and accessed entries will be promoted to.
    head: Option<NonNull<Bucket<K, V>>>,
    /// Pointer to the least recently used entry.
    ///
    /// This is where entries will be evicted from the cache if the capacity is reached.
    tail: Option<NonNull<Bucket<K, V>>>,
}

impl<K, V> LruCache<K, V> {
    pub fn new(capacity: usize) -> Self {
        Self {
            map: HashMap::with_capacity(capacity),
            head: None,
            tail: None,
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
        if self.map.len() == self.map.capacity() {
            self.pop();
        }

        let bucket = NonNull::new(Box::into_raw(Box::new(Bucket {
            value,
            key,
            pointers: UnsafeRefCell::new(Pointers {
                prev: None,
                next: self.head,
            }),
        })))
        .unwrap();

        if let Some(head) = self.head {
            unsafe {
                head.as_ref().pointers.get_mut().prev = Some(bucket);
            }
        }

        self.head = Some(bucket);
        if self.tail.is_none() {
            self.tail = Some(bucket);
        }

        self.map
            .insert(KeyPtr::from_bucket(bucket.as_ptr().cast_const()), bucket);
    }

    /// Returns a reference to a value in the `LruCache`.
    ///
    /// If the value for the given `key` exists the entry is promoted to the most recently used
    /// entry.
    pub fn get<Q>(&mut self, key: Q) -> Option<&V>
    where
        Q: Borrow<K>,
        K: Eq + Hash,
    {
        self.get_mut(key).map(|v| &*v)
    }

    /// Returns a mutable reference to a value in the `LruCache`.
    ///
    /// If the value for the given `key` exists the entry is promoted to the most recently used
    /// entry.
    pub fn get_mut<Q>(&mut self, key: Q) -> Option<&mut V>
    where
        Q: Borrow<K>,
        K: Eq + Hash,
    {
        let mut ptr = *self.map.get(&KeyPtr::from_key(key.borrow()))?;

        // Promote the bucket by placing it at `self.head`.
        unsafe {
            let bucket = ptr.as_mut();
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
            self.head = Some(ptr);

            Some(&mut bucket.value)
        }
    }

    /// Removes the least recently used entry from the `LruCache`.
    pub fn pop(&mut self) -> Option<(K, V)>
    where
        K: Eq + Hash,
    {
        let tail = self.tail?;

        let res = self
            .map
            .remove(&KeyPtr::from_bucket(tail.as_ptr().cast_const()));
        debug_assert_eq!(res, Some(tail));

        unsafe {
            let boxed = Box::from_raw(tail.as_ptr());
            let pointers = boxed.pointers.get_mut();

            match pointers.prev {
                Some(prev) => prev.as_ref().pointers.get_mut().next = None,
                None => self.head = None,
            }

            self.tail = pointers.prev;

            Some((boxed.key, boxed.value))
        }
    }
}

impl<K, V> Drop for LruCache<K, V> {
    fn drop(&mut self) {
        for (_, bucket) in self.map.drain() {
            unsafe {
                drop(Box::from_raw(bucket.as_ptr()));
            }
        }
    }
}

#[derive(Debug)]
struct KeyPtr<K> {
    ptr: *const K,
}

impl<K> KeyPtr<K> {
    fn from_key(key: &K) -> Self {
        Self {
            ptr: key as *const K,
        }
    }

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

struct Bucket<K, V> {
    pointers: UnsafeRefCell<Pointers<K, V>>,
    key: K,
    value: V,
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

    #[test]
    fn lru_cache_insert_with_overflow() {
        let mut cache = LruCache::new(3);
        cache.insert(0, 0);
        cache.insert(1, 1);
        cache.insert(2, 2);
        cache.insert(3, 3);

        assert_eq!(cache.get(0), None);
        assert_eq!(cache.get(1), Some(&1));
        assert_eq!(cache.get(2), Some(&2));
        assert_eq!(cache.get(3), Some(&3));
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
