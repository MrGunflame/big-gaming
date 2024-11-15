use std::cmp;
use std::fmt::{self, Debug, Formatter};
use std::fs::File;
use std::io::ErrorKind;
use std::path::Path;

use game_tracing::trace_span;
use symphonia::core::audio::{AudioBuffer, AudioBufferRef, Signal};
use symphonia::core::codecs::Decoder;
use symphonia::core::conv::FromSample;
use symphonia::core::formats::FormatReader;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::sample::Sample;

use crate::buffer::{Buf, BufMut, Sequential, SequentialView, SequentialViewMut};
use crate::resampler::{self, Resampler};
use crate::sound::Frame;
use crate::sound_data::SoundData;

#[derive(Debug)]
pub struct AudioSource {
    sample_rate: u32,
    buffer: FrameBuffer,
    resampler: Option<Resampler>,
    decode_buffer: FrameBuffer,
    source: Source,
}

impl AudioSource {
    pub fn from_file<P>(path: P) -> Self
    where
        P: AsRef<Path>,
    {
        let file = File::open(path).unwrap();

        let codecs = symphonia::default::get_codecs();
        let probe = symphonia::default::get_probe();
        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        let format_reader = probe
            .format(
                &Default::default(),
                mss,
                &Default::default(),
                &Default::default(),
            )
            .unwrap()
            .format;

        let codec_params = &format_reader.default_track().unwrap().codec_params;
        let sample_rate = codec_params.sample_rate.unwrap();

        let decoder = codecs.make(codec_params, &Default::default()).unwrap();

        Self {
            sample_rate,
            buffer: FrameBuffer::new(),
            resampler: None,
            decode_buffer: FrameBuffer::new(),
            source: Source::Io(IoSource {
                decoder,
                format_reader,
            }),
        }
    }

    pub fn from_data(data: SoundData) -> Self {
        let left = data.frames.iter().map(|f| f.left * data.volume.0);
        let right = data.frames.iter().map(|f| f.right * data.volume.0);

        let mut buf = FrameBuffer::new();
        buf.reserve(data.frames.len());

        let mut dst = buf.spare_capacity_mut();
        for (src, dst) in left.zip(dst.channel_mut(0).unwrap()) {
            *dst = src;
        }

        for (src, dst) in right.zip(dst.channel_mut(1).unwrap()) {
            *dst = src;
        }

        buf.increase_len(data.frames.len());

        Self {
            source: Source::Empty,
            decode_buffer: buf,
            sample_rate: data.sample_rate,
            buffer: FrameBuffer::new(),
            resampler: None,
        }
    }

    /// Sets the sample rate that should be used by this `AudioSource`.
    pub(crate) fn set_sample_rate(&mut self, sample_rate: u32) {
        if sample_rate != self.sample_rate {
            self.resampler = Some(Resampler::new(self.sample_rate, sample_rate));
        }
    }

    /// Reads frames of this source into `buf`. Returns the number of frames written.
    pub(crate) fn read(&mut self, mut buf: &mut [Frame]) -> usize {
        let _span = trace_span!("AudioSource::read").entered();

        // If we samples from the previous packet we flush
        // them first.
        // If they are enough to fill `buf` we don't need
        // to decode any packets.
        let mut frames_written = self.buffer.move_frames_into(buf);
        buf = &mut buf[frames_written..];
        if buf.is_empty() {
            return frames_written;
        }

        loop {
            match self.prepare_next_frame() {
                Ok(()) => (),
                Err(Error::Eof) => return frames_written,
                Err(err) => todo!(),
            }

            let count = self.buffer.move_frames_into(buf);
            buf = &mut buf[count..];
            frames_written += count;

            if buf.is_empty() {
                return frames_written;
            }
        }
    }

    /// Load the next frame into the current buffer.
    ///
    /// May load more than a single frame at once.
    ///
    /// # Errors
    ///
    /// Returns an [`Error::Eof`] if the `AudioSource` has no more frames that can be presented.
    fn prepare_next_frame(&mut self) -> Result<(), Error> {
        match &mut self.resampler {
            Some(resampler) => loop {
                let src = self.decode_buffer.initialized();
                let dst = self.buffer.spare_capacity_mut();

                match resampler.resample(src, dst) {
                    Ok(output) => {
                        self.decode_buffer.remove_frames(output.frames_read);
                        self.buffer.increase_len(output.frames_written);
                        break;
                    }
                    Err(resampler::Error::InputTooSmall(len_required)) => {
                        // This means we need to decode another packet.
                        while self.decode_buffer.len < len_required {
                            self.source.decode_packet(&mut self.decode_buffer)?;
                        }

                        // Try again now that we have a big enough
                        // input buffer.
                        continue;
                    }
                    Err(resampler::Error::OutputTooSmall(len)) => {
                        // Reserve additional space in the output buffer
                        // and try again.
                        self.buffer.reserve(len);
                        continue;
                    }
                }
            },
            None => {
                if self.decode_buffer.is_empty() {
                    self.source.decode_packet(&mut self.decode_buffer)?;
                }

                // If we don't need to resample we only
                // have to copy all decoded frames into
                // the "ready-to-output" buffer.
                let src = self.decode_buffer.initialized();

                self.buffer.reserve(src.num_frames());
                let mut dst = self.buffer.spare_capacity_mut();

                for channel_index in 0..src.num_channels() {
                    let count = src.num_frames();

                    let src = src.channel(channel_index).unwrap();
                    let dst = dst.channel_mut(channel_index).unwrap();
                    dst[..count].copy_from_slice(src);
                }

                let frames_written = src.num_frames();
                self.buffer.increase_len(frames_written);
                self.decode_buffer.remove_frames(frames_written);
            }
        }

        Ok(())
    }
}

fn copy_frames_from_buffer_ref<Dst>(src: &AudioBufferRef<'_>, dst: Dst) -> usize
where
    Dst: BufMut<Sample = f32>,
{
    match src {
        AudioBufferRef::U8(buf) => copy_frames_from_buffer(buf, dst),
        AudioBufferRef::U16(buf) => copy_frames_from_buffer(buf, dst),
        AudioBufferRef::U24(buf) => copy_frames_from_buffer(buf, dst),
        AudioBufferRef::U32(buf) => copy_frames_from_buffer(buf, dst),
        AudioBufferRef::S8(buf) => copy_frames_from_buffer(buf, dst),
        AudioBufferRef::S16(buf) => copy_frames_from_buffer(buf, dst),
        AudioBufferRef::S24(buf) => copy_frames_from_buffer(buf, dst),
        AudioBufferRef::S32(buf) => copy_frames_from_buffer(buf, dst),
        AudioBufferRef::F32(buf) => copy_frames_from_buffer(buf, dst),
        AudioBufferRef::F64(buf) => copy_frames_from_buffer(buf, dst),
    }
}

fn copy_frames_from_buffer<T, Dst>(src: &AudioBuffer<T>, mut dst: Dst) -> usize
where
    f32: FromSample<T>,
    T: Sample,
    Dst: BufMut<Sample = f32>,
{
    match src.spec().channels.count() {
        1 => {
            let dst = dst.channel_mut(0).unwrap();

            debug_assert!(dst.len() >= src.chan(0).len());
            for (sample, dst) in src.chan(0).iter().zip(dst) {
                *dst = f32::from_sample(*sample);
            }
        }
        2 => {
            debug_assert!(dst.channel_mut(0).unwrap().len() >= src.chan(0).len());
            for (src, dst) in src.chan(0).iter().zip(dst.channel_mut(0).unwrap()) {
                *dst = f32::from_sample(*src);
            }

            debug_assert!(dst.channel_mut(1).unwrap().len() >= src.chan(1).len());
            for (src, dst) in src.chan(1).iter().zip(dst.channel_mut(1).unwrap()) {
                *dst = f32::from_sample(*src);
            }
        }
        _ => panic!("unsupported channel config"),
    }

    //let mut frames_written = dst.num_frames();
    // for index in 0..src.spec().channels.count() {
    //     frames_written = frames_written.min(src.chan(index).len());
    // }

    //frames_written
    0
}

#[derive(Debug)]
pub(crate) enum Error {
    /// [`AudioSource`] has finished.
    Eof,
    Decode(symphonia::core::errors::Error),
}

#[derive(Clone, Debug, Default)]
struct FrameBuffer {
    buffer: Sequential<f32>,
    /// Number of frames written in the buffer.
    len: usize,
}

impl FrameBuffer {
    fn new() -> Self {
        Self {
            buffer: Sequential::new(0),
            len: 0,
        }
    }

    /// Returns the number of frames stored in the buffer.
    fn len(&self) -> usize {
        self.len
    }

    /// Returns `true` if the buffer stores no frames.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Reserves capacity for at least `free` number of uninitialized frames at the end.
    fn reserve(&mut self, free: usize) {
        let new_len = self.len + free;

        if self.buffer.num_frames() < new_len {
            self.buffer.resize(new_len);
        }
    }

    /// Moves as many frames as possible from the buffer into `dst`.
    fn move_frames_into(&mut self, dst: &mut [Frame]) -> usize {
        let count = cmp::min(self.len, dst.len());

        for channel in [0, 1] {
            let src = self.buffer.channel_mut(channel).unwrap();

            for (src, dst) in src.iter().zip(dst.iter_mut()).take(count) {
                match channel {
                    // Left
                    0 => dst.left = *src,
                    // Right
                    1 => dst.right = *src,
                    _ => unreachable!(),
                }
            }
        }

        self.remove_frames(count);

        count
    }

    /// Returns the initialized subsection of the buffer.
    fn initialized(&mut self) -> SequentialView<'_, f32> {
        self.buffer.frames_range(..self.len)
    }

    /// Returns the uninitialized subsection of the buffer.
    fn spare_capacity_mut(&mut self) -> SequentialViewMut<'_, f32> {
        self.buffer.frames_range_mut(self.len..)
    }

    /// Increases the len by `extra`, marking the last `extra` elements as initialized.
    fn increase_len(&mut self, extra: usize) {
        self.len += extra;
    }

    /// Removes the first `count` frames.
    fn remove_frames(&mut self, count: usize) {
        for frames in self.buffer.channels_mut() {
            // Remove the first `count` frames from `self.buffer`
            // by shifting all elements starting at index `count` to the left.
            frames.copy_within(count.., 0);
        }

        self.len -= count;
    }
}

#[derive(Debug)]
enum Source {
    Io(IoSource),
    /// A source that will never yield any frames.
    ///
    /// This `Source` only returns [`Error::Eof`].
    Empty,
}

impl Source {
    /// Decodes a single packet from this `Source` into the given [`FrameBuffer`].
    ///
    /// # Errors
    ///
    /// Returns an [`Error::Eof`] when the last packet has been decoded and this `Source` will
    /// never yield another packet.
    fn decode_packet(&mut self, dst: &mut FrameBuffer) -> Result<(), Error> {
        match self {
            Source::Empty => Err(Error::Eof),
            Source::Io(source) => match source.format_reader.next_packet() {
                Ok(packet) => {
                    let buffer = source.decoder.decode(&packet).map_err(Error::Decode)?;

                    dst.reserve(buffer.frames());
                    copy_frames_from_buffer_ref(&buffer, &mut dst.spare_capacity_mut());
                    dst.increase_len(buffer.frames());

                    Ok(())
                }
                Err(err) => match err {
                    symphonia::core::errors::Error::IoError(err)
                        if err.kind() == ErrorKind::UnexpectedEof =>
                    {
                        Err(Error::Eof)
                    }
                    err => Err(Error::Decode(err)),
                },
            },
        }
    }
}

/// A source backed by some IO object, usually a file or similar.
struct IoSource {
    decoder: Box<dyn Decoder>,
    format_reader: Box<dyn FormatReader>,
}

impl Debug for IoSource {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("IoSource").finish_non_exhaustive()
    }
}
