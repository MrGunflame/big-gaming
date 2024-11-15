use core::slice;
use std::mem::MaybeUninit;
use std::ops::{Bound, Range, RangeBounds};

use bytemuck::Pod;

const CHANNELS: usize = 2;

#[derive(Clone, Debug)]
pub struct Sequential<T> {
    buffer: Vec<T>,
    channels: usize,
    frames: usize,
}

impl<T> Sequential<T>
where
    T: Pod,
{
    pub fn new(frames: usize) -> Self {
        let mut buffer = vec![T::zeroed(); frames * CHANNELS];

        Self {
            buffer,
            channels: CHANNELS,
            frames,
        }
    }

    pub fn resize(&mut self, frames: usize) {
        if self.frames >= frames {
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

    pub fn channel(&self, channel: usize) -> Option<&[T]> {
        let range = self.channel_range(channel)?;
        Some(&self.buffer[range])
    }

    pub fn channel_mut(&mut self, channel: usize) -> Option<&mut [T]> {
        let range = self.channel_range(channel)?;
        Some(&mut self.buffer[range])
    }

    pub fn channel_mut2(
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

        // range0 comes before range1
        if range0.end >= range1.start {
            let (lhs, rhs) = self.buffer.split_at_mut(range0.end);

            let left = &mut lhs[range0.start..];
            debug_assert_eq!(left.len(), range0.len());

            let right = &mut rhs[range1.start - range0.end..range1.start - range0.end + range1.end];
            debug_assert_eq!(right.len(), range1.len());
            Some((left, right))
        } else {
            let (lhs, rhs) = self.buffer.split_at_mut(range1.end);

            let left = &mut lhs[range0.start - range1.end..range0.start - range1.end + range0.end];
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

    pub fn num_frames(&self) -> usize {
        self.frames
    }

    pub fn frames_range<R>(&self, range: R) -> SequentialView<'_, T>
    where
        R: RangeBounds<usize>,
    {
        let range = self.subslice_frames_range(range);
        if range.start > range.end || range.end > self.frames {
            panic!("range out of bounds");
        }

        SequentialView { inner: self, range }
    }

    pub fn frames_range_mut<R>(&mut self, range: R) -> SequentialViewMut<'_, T>
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

    pub fn channels_mut(&mut self) -> ChannelsMut<'_, T> {
        ChannelsMut {
            buffer: &mut self.buffer,
            num_channels: self.channels,
            num_frames: self.frames,
            index: 0,
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

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Incompatible;

pub(crate) trait Buf {
    type Sample;

    fn num_frames(&self) -> usize;

    fn num_channels(&self) -> usize;

    fn channel(&self, channel: usize) -> Option<&[Self::Sample]>;
}

pub(crate) trait BufMut: Buf {
    fn channel_mut(&mut self, channel: usize) -> Option<&mut [Self::Sample]>;

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

pub struct ChannelsMut<'a, T> {
    buffer: &'a mut [T],
    num_channels: usize,
    num_frames: usize,
    index: usize,
}
impl<'a, T> Iterator for ChannelsMut<'a, T> {
    type Item = &'a mut [T];

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.num_channels {
            return None;
        }

        self.index += 1;
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
