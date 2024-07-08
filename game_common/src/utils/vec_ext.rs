/// Extension functions for [`Vec`].
pub trait VecExt<T> {
    /// Extends the `Vec` by inserting `src`, starting at `index` while pushing all other elements
    /// to the right.
    ///
    /// # Panics
    ///
    /// Pancis if `index` is out of bounds.
    fn extend_at(&mut self, index: usize, src: &[T])
    where
        T: Copy;

    /// Extens the `Vec` by inserting `src`, starting at `index` while pushing all other elements
    /// to the right. This function does not check whether `index` is valid.
    ///
    /// # Safety
    ///
    /// `index` must be in bounds, i.e. `index <= self.len()` must always be true.
    unsafe fn extend_at_unchecked(&mut self, index: usize, src: &[T])
    where
        T: Copy;
}

impl<T> VecExt<T> for Vec<T> {
    fn extend_at(&mut self, index: usize, src: &[T])
    where
        T: Copy,
    {
        // Do bounds checking.
        #[cold]
        #[inline(never)]
        #[track_caller]
        fn assert_failed(index: usize, len: usize) -> ! {
            panic!("insertion index (is {index}) should be <= len (is {len})");
        }

        if index > self.len() {
            assert_failed(index, self.len());
        }

        unsafe {
            self.extend_at_unchecked(index, src);
        }
    }

    unsafe fn extend_at_unchecked(&mut self, index: usize, src: &[T])
    where
        T: Copy,
    {
        // Grow the capacity enough to hold the new slice.
        // We are relying on having enough space capacity
        // in the `Vec` before we can copy the regions.
        self.reserve(src.len());

        let len = self.len();
        let ptr = self.as_mut_ptr();

        unsafe {
            // Copy the region after `index` to the end of the vector.
            // Note that the regions may overlap, therefore we must
            // use `copy` instead of `copy_nonoverlapping`.
            let trailer_len = len - index;
            let trailer_src = ptr.add(index).cast_const();
            let trailer_dst = ptr.add(index).add(src.len());
            core::ptr::copy(trailer_src, trailer_dst, trailer_len);

            let dst = ptr.add(index);
            core::ptr::copy_nonoverlapping(src.as_ptr(), dst, src.len());

            // All regions are now initialized.
            self.set_len(len + src.len());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::VecExt;

    #[test]
    fn extend_at_short_src() {
        let mut vec = vec![0, 1, 2, 3, 4, 5, 6];
        vec.extend_at(3, &[7, 8, 9]);
        assert_eq!(vec, [0, 1, 2, 7, 8, 9, 3, 4, 5, 6]);
    }

    #[test]
    fn extend_at_long_src() {
        let mut vec = vec![0, 1, 2];
        vec.extend_at(1, &[3, 4, 5, 6, 7, 8, 9]);
        assert_eq!(vec, [0, 3, 4, 5, 6, 7, 8, 9, 1, 2]);
    }

    #[test]
    #[should_panic]
    fn extend_at_out_of_bounds() {
        let mut vec = vec![0];
        vec.extend_at(2, &[]);
    }
}
