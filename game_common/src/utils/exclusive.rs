use std::fmt::{self, Debug, Formatter};

// Note that we deliberately do not derive any traits that with a `&self`
// receiver, which would break the "only single-reference" invariant.
#[repr(transparent)]
#[derive(Default)]
pub struct Exclusive<T>
where
    T: ?Sized,
{
    inner: T,
}

impl<T> Exclusive<T> {
    #[inline]
    pub const fn new(t: T) -> Self {
        Self { inner: t }
    }

    #[inline]
    pub fn into_inner(self) -> T {
        self.inner
    }

    #[inline]
    pub fn get_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}

impl<T> Debug for Exclusive<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Exclusive").finish_non_exhaustive()
    }
}

// Exclusive only allows a single reference to exist at any time.
unsafe impl<T: ?Sized> Sync for Exclusive<T> {}
