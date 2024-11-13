use std::fmt::Debug;
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

use crate::sound::Frame;

pub struct AudioSource {
    decoder: Box<dyn Decoder>,
    sample_rate: u32,
    format_reader: Box<dyn FormatReader>,
    buffer: Vec<Frame>,
    buffer_len: usize,
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
            decoder,
            sample_rate,
            format_reader,
            buffer: Vec::new(),
            buffer_len: 0,
        }
    }

    pub fn read(&mut self, mut buf: &mut [Frame]) -> usize {
        let _span = trace_span!("AudioSource::read").entered();

        let mut bytes_written = 0;

        // If we samples from the previous packet we flush
        // them first.
        // If they are enough to fill `buf` we don't need
        // to decode any packets.
        let count = usize::min(self.buffer_len, buf.len());
        buf[..count].copy_from_slice(&self.buffer[..count]);
        self.buffer_len -= count;
        for _ in 0..count {
            self.buffer.remove(0);
        }

        buf = &mut buf[count..];
        if buf.is_empty() {
            return count;
        }

        loop {
            match self.format_reader.next_packet() {
                Ok(packet) => {
                    let buffer = self.decoder.decode(&packet).unwrap();
                    if self.buffer.len() < buffer.frames() {
                        let new_len = self.buffer_len + buffer.frames();
                        self.buffer.resize(new_len, Frame::EQUILIBRIUM);
                    }

                    match copy_frames_from_buffer_ref(&buffer, &mut self.buffer[self.buffer_len..])
                    {
                        0 => return bytes_written,
                        n => {
                            self.buffer_len += n;

                            let count = usize::min(self.buffer_len, buf.len());
                            buf[..count].copy_from_slice(&self.buffer[..count]);
                            self.buffer_len -= count;
                            for _ in 0..count {
                                self.buffer.remove(0);
                            }
                            bytes_written += count;

                            buf = &mut buf[count..];
                            if buf.is_empty() {
                                return bytes_written;
                            }
                        }
                    }
                }
                Err(err) => match err {
                    symphonia::core::errors::Error::IoError(err)
                        if err.kind() == ErrorKind::UnexpectedEof =>
                    {
                        return bytes_written;
                    }
                    err => panic!("{}", err),
                },
            }
        }
    }
}

impl Debug for AudioSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AudioSource")
            .field("sample_rate", &self.sample_rate)
            .finish_non_exhaustive()
    }
}

fn copy_frames_from_buffer_ref(src: &AudioBufferRef<'_>, dst: &mut [Frame]) -> usize {
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

fn copy_frames_from_buffer<T>(src: &AudioBuffer<T>, dst: &mut [Frame]) -> usize
where
    f32: FromSample<T>,
    T: Sample,
{
    match src.spec().channels.count() {
        1 => {
            for (sample, dst) in src.chan(0).iter().zip(dst.iter_mut()) {
                *dst = Frame::from_mono(f32::from_sample(*sample));
            }
        }
        2 => {
            for ((left, right), dst) in src
                .chan(0)
                .iter()
                .zip(src.chan(1).iter())
                .zip(dst.iter_mut())
            {
                *dst = Frame {
                    left: f32::from_sample(*left),
                    right: f32::from_sample(*right),
                };
            }
        }
        _ => panic!("unsupported channel config"),
    }

    let mut frames_written = dst.len();
    for index in 0..src.spec().channels.count() {
        frames_written = frames_written.min(src.chan(index).len());
    }

    frames_written
}

#[derive(Debug)]
pub enum Error {
    /// [`AudioSource`] has finished.
    Eof,
}
