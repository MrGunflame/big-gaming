use std::cell::UnsafeCell;
use std::marker::PhantomData;
use std::ptr::NonNull;

/// A type which can be used as an [`LinkedList`] element.
///
/// # Safety
///
/// The returned pointer to [`Pointers`] must be valid for the same duration as `ptr` under the
/// assumption that `ptr` points to a valid `Self` instance.
///
/// A valid pointer has the following properties:
/// - It is well-aligned
/// - It can be dereferenceable
/// - It points to an initialized `Self` value
pub(crate) unsafe trait Link {
    /// Returns the intrusive `Pointers` of the element.
    ///
    /// The [`Pointers`] returned by this function are valid for the same lifetime as `ptr`.
    ///
    /// # Safety
    ///
    /// While the [`Pointers`] returned by this function are dereferenced, the object must not
    /// occur any mutation.
    unsafe fn pointers(ptr: NonNull<Self>) -> NonNull<Pointers<Self>>;
}

#[derive(Debug)]
#[repr(transparent)]
pub(crate) struct Pointers<T>
where
    T: ?Sized,
{
    inner: UnsafeCell<PointersInner<T>>,
}

impl<T> Pointers<T> {
    #[inline]
    pub const fn new() -> Self {
        Self {
            inner: UnsafeCell::new(PointersInner {
                next: None,
                prev: None,
            }),
        }
    }
}

impl<T> Default for Pointers<T> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Default)]
#[repr(C)]
struct PointersInner<T>
where
    T: ?Sized,
{
    next: Option<NonNull<T>>,
    prev: Option<NonNull<T>>,
}

/// An instrusive linked.
#[derive(Debug)]
pub(crate) struct LinkedList<T> {
    head: Option<NonNull<T>>,
    tail: Option<NonNull<T>>,
    _marker: PhantomData<*const T>,
    #[cfg(debug_assertions)]
    len: usize,
}

impl<T> LinkedList<T>
where
    T: Link,
{
    /// Creates a new, empty `LinkedList`.
    pub const fn new() -> Self {
        Self {
            head: None,
            tail: None,
            _marker: PhantomData,
            #[cfg(debug_assertions)]
            len: 0,
        }
    }

    /// Pushes a new element to the back of the `LinkedList`.
    ///
    /// # Safety
    ///
    /// The `NonNull<T>` pointer must be well-formed and point to an initialized `T`. The pushed
    /// pointer must stay valid until it is removed with [`remove`].
    ///
    /// [`remove`]: Self::remove
    #[inline]
    pub unsafe fn push_back(&mut self, ptr: NonNull<T>) {
        {
            let pointers = unsafe { &mut *T::pointers(ptr).as_ref().inner.get() };
            pointers.next = None;
            pointers.prev = self.tail;
        }

        match self.tail {
            Some(tail) => {
                let tail_pointers = unsafe { &mut *T::pointers(tail).as_ref().inner.get() };
                tail_pointers.next = Some(ptr);
            }
            None => self.head = Some(ptr),
        }

        self.tail = Some(ptr);

        #[cfg(debug_assertions)]
        {
            self.len += 1;
            if self.len > 1 {
                assert_ne!(self.head, self.tail);
            }
        }
    }

    /// Removes the element from the `LinkedList`.
    ///
    /// # Safety
    ///
    /// The `NonNull<T>` pointer must be well-formed and point to an initialized `T`. The pointer
    /// must have been previously inserted into the `LinkedList`.
    #[inline]
    pub unsafe fn remove(&mut self, ptr: NonNull<T>) {
        #[cfg(debug_assertions)]
        {
            self.len -= 1;
        }

        let pointers = unsafe { &mut *T::pointers(ptr).as_ref().inner.get() };

        match pointers.next {
            Some(next) => {
                let next_pointers = unsafe { &mut *T::pointers(next).as_ref().inner.get() };
                next_pointers.prev = pointers.prev;
            }
            None => self.tail = pointers.prev,
        }

        match pointers.prev {
            Some(prev) => {
                let prev_pointers = unsafe { &mut *T::pointers(prev).as_ref().inner.get() };
                prev_pointers.next = pointers.next;
            }
            None => self.head = pointers.next,
        }
    }

    /// Returns the pointer to the head element of the `LinkedList`.
    #[inline]
    pub fn head(&self) -> Option<NonNull<T>> {
        self.head
    }
}

#[cfg(debug_assertions)]
impl<T> Drop for LinkedList<T> {
    fn drop(&mut self) {
        assert_eq!(self.len, 0);
        assert_eq!(self.head, None);
        assert_eq!(self.tail, None);
    }
}

unsafe impl<T> Send for LinkedList<T> where T: Link + Send {}
unsafe impl<T> Sync for LinkedList<T> where T: Link + Sync {}

#[cfg(test)]
mod tests {
    use std::ptr::NonNull;

    use super::{Link, LinkedList, Pointers};

    #[derive(Debug, Default)]
    #[repr(transparent)]
    struct Node(Pointers<Self>);

    unsafe impl Link for Node {
        unsafe fn pointers(ptr: NonNull<Self>) -> NonNull<Pointers<Self>> {
            // `Node` is `#[repr(transparent)]` therefore we can case a
            // `*Node` to a `*Pointers`.
            ptr.cast()
        }
    }

    #[test]
    fn linked_list_push_back() {
        let mut list = LinkedList::new();

        let mut node0 = Node::default();
        let mut node1 = Node::default();
        let mut node2 = Node::default();

        unsafe {
            list.push_back((&node0).into());
        }

        assert_eq!(list.head, Some((&node0).into()));
        assert_eq!(list.tail, Some((&node0).into()));

        assert_eq!(node0.0.inner.get_mut().next, None);
        assert_eq!(node0.0.inner.get_mut().prev, None);

        unsafe {
            list.push_back((&node1).into());
        }

        assert_eq!(list.head, Some((&node0).into()));
        assert_eq!(list.tail, Some((&node1).into()));

        assert_eq!(node0.0.inner.get_mut().next, Some((&node1).into()));
        assert_eq!(node0.0.inner.get_mut().prev, None);

        assert_eq!(node1.0.inner.get_mut().next, None);
        assert_eq!(node1.0.inner.get_mut().prev, Some((&node0).into()));

        unsafe {
            list.push_back((&node2).into());
        }

        assert_eq!(list.head, Some((&node0).into()));
        assert_eq!(list.tail, Some((&node2).into()));

        assert_eq!(node0.0.inner.get_mut().next, Some((&node1).into()));
        assert_eq!(node0.0.inner.get_mut().prev, None);

        assert_eq!(node1.0.inner.get_mut().next, Some((&node2).into()));
        assert_eq!(node1.0.inner.get_mut().prev, Some((&node0).into()));

        assert_eq!(node2.0.inner.get_mut().next, None);
        assert_eq!(node2.0.inner.get_mut().prev, Some((&node1).into()));

        core::mem::forget(list);
    }

    #[test]
    fn linked_list_remove_head() {
        let mut list = LinkedList::new();

        let node0 = Node::default();
        let mut node1 = Node::default();
        let mut node2 = Node::default();

        for node in [&node0, &node1, &node2] {
            unsafe {
                list.push_back(node.into());
            }
        }

        unsafe {
            list.remove((&node0).into());
        }

        assert_eq!(list.head, Some((&node1).into()));
        assert_eq!(list.tail, Some((&node2).into()));

        assert_eq!(node1.0.inner.get_mut().next, Some((&node2).into()));
        assert_eq!(node1.0.inner.get_mut().prev, None);

        assert_eq!(node2.0.inner.get_mut().next, None);
        assert_eq!(node2.0.inner.get_mut().prev, Some((&node1).into()));

        core::mem::forget(list);
    }

    #[test]
    fn linked_list_remove_tail() {
        let mut list = LinkedList::new();

        let mut node0 = Node::default();
        let mut node1 = Node::default();
        let node2 = Node::default();

        for node in [&node0, &node1, &node2] {
            unsafe {
                list.push_back(node.into());
            }
        }

        unsafe {
            list.remove((&node2).into());
        }

        assert_eq!(list.head, Some((&node0).into()));
        assert_eq!(list.tail, Some((&node1).into()));

        assert_eq!(node0.0.inner.get_mut().next, Some((&node1).into()));
        assert_eq!(node0.0.inner.get_mut().prev, None);

        assert_eq!(node1.0.inner.get_mut().next, None);
        assert_eq!(node1.0.inner.get_mut().prev, Some((&node0).into()));

        core::mem::forget(list);
    }

    #[test]
    fn linked_list_remove_middle() {
        let mut list = LinkedList::new();

        let mut node0 = Node::default();
        let node1 = Node::default();
        let mut node2 = Node::default();

        for node in [&node0, &node1, &node2] {
            unsafe {
                list.push_back(node.into());
            }
        }

        unsafe {
            list.remove((&node1).into());
        }

        assert_eq!(list.head, Some((&node0).into()));
        assert_eq!(list.tail, Some((&node2).into()));

        assert_eq!(node0.0.inner.get_mut().next, Some((&node2).into()));
        assert_eq!(node0.0.inner.get_mut().prev, None);

        assert_eq!(node2.0.inner.get_mut().next, None);
        assert_eq!(node2.0.inner.get_mut().prev, Some((&node0).into()));

        core::mem::forget(list);
    }
}
