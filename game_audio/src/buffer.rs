use std::iter::FusedIterator;
use std::ops::{Bound, Range, RangeBounds};

use bytemuck::Pod;

const CHANNELS: usize = 2;

/// A buffer storing the frames of each channel sequentially.
///
/// # Example
///
/// A buffer with channels `A` and `B` of 3 frames will look as follows:
/// ```text
/// +---+---+---+---+---+---+
/// | A | A | A | B | B | B |
/// +---+---+---+---+---+---+
/// ```
#[derive(Clone, Debug)]
pub(crate) struct Sequential<T> {
    buffer: Vec<T>,
    channels: usize,
    frames: usize,
}

impl<T> Sequential<T>
where
    T: Pod,
{
    /// Creates a new `Sequential` buffer with the given number of `frames`.
    pub(crate) fn new(frames: usize) -> Self {
        let buffer = vec![T::zeroed(); frames * CHANNELS];

        Self {
            buffer,
            channels: CHANNELS,
            frames,
        }
    }

    /// Resizes the buffer to the given number of `frames`.
    ///
    /// If the new number of frames is less than the current number of frames, each channel will be
    /// truncated to the new number of frames.
    ///
    /// If the new number of frames is greater than the current number of frames, each channel will
    /// be extended at the end and the extended space will be filled with an unspecified frame value.
    pub(crate) fn resize(&mut self, frames: usize) {
        if self.frames == frames {
            return;
        }

        if self.frames > frames {
            // Truncate every channel to `frames` len.
            let remove_count = self.frames - frames;

            // By reversing the iterator all ranges stay valid.
            for channel in (0..self.channels).rev() {
                let range = self.channel_range(channel).unwrap();
                let drain = range.start + remove_count..range.end;
                self.buffer.drain(drain);
            }
        } else {
            // Extend every channel to `frames` len.

            // Resize the buffer to the correct size.
            // This will add new memory for the samples at
            // the end of the buffer.
            let new_len = frames * self.channels;
            self.buffer.resize(new_len, T::zeroed());

            // Copy all samples into the correct new location, starting
            // with the last channel.
            // This means previous channels can use the memory of the next
            // channel.
            //
            // For example, resizing a 2 channel 2 frame buffer to 3 frames:
            //
            // +---+---+---+---+  (resize buffer)   +---+---+---+---+---+---+
            // | 0 | 1 | 0 | 1 | =================> | 0 | 1 | 0 | 1 | Z | Z |
            // +---+---+---+---+                    +---+---+---+---+---+---+
            //
            // +---+---+---+---+---+---+  (copy channel 1)   +---+---+---+---+---+---+
            // | 0 | 1 | 0 | 1 | Z | Z | ==================> | 0 | 1 | 0 | 0 | 1 | 2 |
            // +---+---+---+---+---+---+                     +---+---+---+---+---+---+
            //
            // +---+---+---+---+---+---+  (copy channel 0)   +---+---+---+---+---+---+
            // | 0 | 1 | 0 | 0 | 1 | 2 | ==================> | 0 | 1 | 2 | 0 | 1 | 2 |
            // +---+---+---+---+---+---+                     +---+---+---+---+---+---+
            //
            for channel in (0..self.channels).rev() {
                let old_range = self.frames * channel..self.frames * (channel + 1);
                let new_range = frames * channel..frames * (channel + 1);
                self.buffer.copy_within(old_range, new_range.start);
            }
        }

        self.frames = frames;
        debug_assert_eq!(self.buffer.len(), self.frames * self.channels);
    }

    /// Return a reference to the frames of the channel with the given index `channel`. Returns
    /// `None` if the channel does not exist.
    pub(crate) fn channel(&self, channel: usize) -> Option<&[T]> {
        let range = self.channel_range(channel)?;
        Some(&self.buffer[range])
    }

    /// Returns a mutable reference to the frames of the channel with the given index `channel`.
    /// Returns `None` if the channel does not exist.
    pub(crate) fn channel_mut(&mut self, channel: usize) -> Option<&mut [T]> {
        let range = self.channel_range(channel)?;
        Some(&mut self.buffer[range])
    }

    /// Returns mutable references to the frames of two distinct channels. Returns `None` if any
    /// of the channels do not exist, or if both are the same.
    pub(crate) fn channel_mut2(
        &mut self,
        channel0: usize,
        channel1: usize,
    ) -> Option<(&mut [T], &mut [T])> {
        let range0 = self.channel_range(channel0)?;
        let range1 = self.channel_range(channel1)?;
        // Ranges must not be equal.
        if range0 == range1 {
            return None;
        }

        if range0.end >= range1.start {
            let (lhs, rhs) = self.buffer.split_at_mut(range0.end);

            let left = &mut lhs[range0.start..];
            debug_assert_eq!(left.len(), range0.len());

            let rhs_start = range1.start - range0.end;
            let rhs_end = range1.start - range0.end + (range1.end - range1.start);

            let right = &mut rhs[rhs_start..rhs_end];
            debug_assert_eq!(right.len(), range1.len());
            Some((left, right))
        } else {
            let (lhs, rhs) = self.buffer.split_at_mut(range1.end);

            let lhs_start = range0.start - range1.end;
            let lhs_end = range0.start - range1.end + (range0.end - range0.start);

            let left = &mut lhs[lhs_start..lhs_end];
            debug_assert_eq!(left.len(), range0.len());

            let right = &mut rhs[range1.start..];
            debug_assert_eq!(right.len(), range1.len());
            Some((left, right))
        }
    }

    fn channel_range(&self, channel: usize) -> Option<Range<usize>> {
        if channel >= self.channels {
            None
        } else {
            Some(self.frames * channel..self.frames * (channel + 1))
        }
    }

    /// Returns the number of frames stored in each channel.
    pub(crate) fn num_frames(&self) -> usize {
        self.frames
    }

    /// Returns a view into this buffer that only contains the frames within the given range
    /// for each channel.
    ///
    /// # Panics
    ///
    /// Panics if the range is out of bounds for this buffer.
    pub(crate) fn frames_range<R>(&self, range: R) -> SequentialView<'_, T>
    where
        R: RangeBounds<usize>,
    {
        let range = self.subslice_frames_range(range);
        if range.start > range.end || range.end > self.frames {
            panic!("range out of bounds");
        }

        SequentialView { inner: self, range }
    }

    /// Returns a mutable view into this buffer that only contains the frames within the given
    /// range for each channel.
    ///
    /// # Panics
    ///
    /// Panics if the range is out of bounds of this buffer.
    pub(crate) fn frames_range_mut<R>(&mut self, range: R) -> SequentialViewMut<'_, T>
    where
        R: RangeBounds<usize>,
    {
        let range = self.subslice_frames_range(range);
        if range.start > range.end || range.end > self.frames {
            panic!("range out of bounds");
        }

        SequentialViewMut { inner: self, range }
    }

    fn subslice_frames_range<R>(&self, range: R) -> Range<usize>
    where
        R: RangeBounds<usize>,
    {
        let start = match range.start_bound() {
            Bound::Included(index) => *index,
            // FIXME: Handle overflow
            Bound::Excluded(index) => *index + 1,
            Bound::Unbounded => 0,
        };

        let end = match range.end_bound() {
            // FIXME: Handle overflow
            Bound::Included(index) => *index + 1,
            Bound::Excluded(index) => *index,
            Bound::Unbounded => self.frames,
        };

        Range { start, end }
    }

    /// Returns an iterator yielding the frames of all channels mutably.
    pub(crate) fn channels_mut(&mut self) -> ChannelsMut<'_, T> {
        ChannelsMut {
            buffer: &mut self.buffer,
            num_frames: self.frames,
            channels_remaining: self.channels,
        }
    }
}

impl<T> Default for Sequential<T>
where
    T: Pod,
{
    fn default() -> Self {
        Self::new(0)
    }
}

impl<T> Buf for Sequential<T>
where
    T: Pod,
{
    type Sample = T;

    fn num_channels(&self) -> usize {
        self.channels
    }

    fn channel(&self, channel: usize) -> Option<&[Self::Sample]> {
        self.channel(channel)
    }

    fn num_frames(&self) -> usize {
        self.frames
    }
}

impl<T> BufMut for Sequential<T>
where
    T: Pod,
{
    fn channel_mut(&mut self, channel: usize) -> Option<&mut [Self::Sample]> {
        self.channel_mut(channel)
    }

    fn channel_mut2(
        &mut self,
        channel0: usize,
        channel1: usize,
    ) -> Option<(&mut [Self::Sample], &mut [Self::Sample])> {
        self.channel_mut2(channel0, channel1)
    }
}

/// A view into a limited range of frames of a [`Sequential`] buffer.
///
/// Returned by [`frames_range`].
///
/// [`frames_range`]: Sequential::frames_range
#[derive(Clone, Debug)]
pub(crate) struct SequentialView<'a, T> {
    inner: &'a Sequential<T>,
    range: Range<usize>,
}

impl<'a, T> Buf for SequentialView<'a, T>
where
    T: Pod,
{
    type Sample = T;

    fn num_frames(&self) -> usize {
        self.range.end - self.range.start
    }

    fn num_channels(&self) -> usize {
        self.inner.channels
    }

    fn channel(&self, channel: usize) -> Option<&[Self::Sample]> {
        let range = self.range.clone();
        match self.inner.channel(channel) {
            Some(slice) => Some(&slice[range]),
            None => None,
        }
    }
}

/// A mutable view into a limited range of frames of a [`Sequential`] buffer.
///
/// Returned by [`frames_range`].
///
/// [`frames_range`]: Sequential::frames_range_mut
#[derive(Debug)]
pub(crate) struct SequentialViewMut<'a, T> {
    inner: &'a mut Sequential<T>,
    range: Range<usize>,
}

impl<'a, T> Buf for SequentialViewMut<'a, T>
where
    T: Pod,
{
    type Sample = T;

    fn num_frames(&self) -> usize {
        self.range.end - self.range.start
    }

    fn num_channels(&self) -> usize {
        self.inner.channels
    }

    fn channel(&self, channel: usize) -> Option<&[Self::Sample]> {
        match self.inner.channel(channel) {
            Some(slice) => Some(&slice[self.range.clone()]),
            None => None,
        }
    }
}

impl<'a, T> BufMut for SequentialViewMut<'a, T>
where
    T: Pod,
{
    fn channel_mut(&mut self, channel: usize) -> Option<&mut [Self::Sample]> {
        match self.inner.channel_mut(channel) {
            Some(slice) => Some(&mut slice[self.range.clone()]),
            None => None,
        }
    }

    fn channel_mut2(
        &mut self,
        channel0: usize,
        channel1: usize,
    ) -> Option<(&mut [Self::Sample], &mut [Self::Sample])> {
        match self.inner.channel_mut2(channel0, channel1) {
            Some((slice0, slice1)) => Some((
                &mut slice0[self.range.clone()],
                &mut slice1[self.range.clone()],
            )),
            None => None,
        }
    }
}

/// An iterator yielding the frames of each channel mutably.
///
/// Returned by [`channels`].
///
/// [`channels`]: Sequential::channels
pub(crate) struct ChannelsMut<'a, T> {
    buffer: &'a mut [T],
    num_frames: usize,
    channels_remaining: usize,
}

impl<'a, T> Iterator for ChannelsMut<'a, T> {
    type Item = &'a mut [T];

    fn next(&mut self) -> Option<Self::Item> {
        if self.channels_remaining == 0 {
            return None;
        }

        self.channels_remaining -= 1;
        let end = self.num_frames;

        let (frames, rem) = self.buffer.split_at_mut(end);

        // Upgrade both slices from the local scoped lifetime `'1`
        // (`fn next(&'1 mut self)`) to `'a`.
        // This is safe because lifetime of `self.buffer` is bound to `'a`,
        // so all subslices are also bound to `'a`.
        // Since `rem` becomes the new `self.buffer` slice and with that `frames`
        // is permanently inaccessible for future calls, no overlaps with other
        // slices is possible.
        let rem = unsafe { core::mem::transmute::<&mut [T], &'a mut [T]>(rem) };
        let frames = unsafe { core::mem::transmute::<&mut [T], &'a mut [T]>(frames) };

        self.buffer = rem;

        Some(frames)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len(), Some(self.len()))
    }
}

impl<'a, T> ExactSizeIterator for ChannelsMut<'a, T> {
    fn len(&self) -> usize {
        self.channels_remaining
    }
}

impl<'a, T> FusedIterator for ChannelsMut<'a, T> {}

/// A buffer type that exposes access to audio frames.
pub(crate) trait Buf {
    /// The underlying type if the buffer.
    type Sample;

    /// Returns the number of frames in this buffer.
    fn num_frames(&self) -> usize;

    /// Returns the number of channels in this buffer.
    fn num_channels(&self) -> usize;

    /// Returns a reference to the frames of the channel with the given index `channel`. Returns
    /// `None` if the channel does not exist.
    fn channel(&self, channel: usize) -> Option<&[Self::Sample]>;
}

/// A buffer type that exposes access to audio frames mutably.
pub(crate) trait BufMut: Buf {
    /// Returns a mutable reference to the frames of the channel with the given index `channel`.
    /// Returns `None` if the channel does not exist.
    fn channel_mut(&mut self, channel: usize) -> Option<&mut [Self::Sample]>;

    /// Returns mutable references to the frames of two distinct channels. Returns `None` if any
    /// of the channels do not exist, or if both are the same.
    fn channel_mut2(
        &mut self,
        channel0: usize,
        channel1: usize,
    ) -> Option<(&mut [Self::Sample], &mut [Self::Sample])>;
}

impl<T> Buf for &T
where
    T: Buf,
{
    type Sample = T::Sample;

    #[inline]
    fn channel(&self, channel: usize) -> Option<&[Self::Sample]> {
        T::channel(self, channel)
    }

    #[inline]
    fn num_channels(&self) -> usize {
        T::num_channels(self)
    }

    #[inline]
    fn num_frames(&self) -> usize {
        T::num_frames(self)
    }
}

impl<T> Buf for &mut T
where
    T: Buf,
{
    type Sample = T::Sample;

    #[inline]
    fn num_frames(&self) -> usize {
        T::num_frames(self)
    }

    #[inline]
    fn num_channels(&self) -> usize {
        T::num_channels(self)
    }

    #[inline]
    fn channel(&self, channel: usize) -> Option<&[Self::Sample]> {
        T::channel(self, channel)
    }
}

impl<T> BufMut for &mut T
where
    T: BufMut,
{
    #[inline]
    fn channel_mut(&mut self, channel: usize) -> Option<&mut [Self::Sample]> {
        T::channel_mut(self, channel)
    }

    #[inline]
    fn channel_mut2(
        &mut self,
        channel0: usize,
        channel1: usize,
    ) -> Option<(&mut [Self::Sample], &mut [Self::Sample])> {
        T::channel_mut2(self, channel0, channel1)
    }
}

#[cfg(test)]
mod tests {
    use super::Sequential;

    #[test]
    fn sequential_channels_mut() {
        let mut buffer = Sequential::<i32>::new(128);
        buffer.channel_mut(0).unwrap().fill(1);
        buffer.channel_mut(1).unwrap().fill(2);

        // Collect all slice references into `channels`.
        // This helps miri detect potential overlapping
        // slices.
        let mut channels = Vec::new();
        for channel in buffer.channels_mut() {
            channels.push(channel);
        }

        for (index, channel) in channels.iter().enumerate() {
            assert_eq!(*channel, vec![index as i32 + 1; 128]);
        }
    }
}
