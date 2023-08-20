use std::{fs::File, path::Path};

use symphonia::core::{
    audio::{AudioBuffer, AudioBufferRef, Signal},
    conv::FromSample,
    io::MediaSourceStream,
    sample::Sample,
};

use crate::sound::Frame;

pub struct SoundData {
    pub(crate) frames: Vec<Frame>,
    pub(crate) sample_rate: u32,
}

impl SoundData {
    pub fn from_file<P>(path: P) -> Self
    where
        P: AsRef<Path>,
    {
        let media_source = File::open(path).unwrap();

        let codecs = symphonia::default::get_codecs();
        let probe = symphonia::default::get_probe();
        let mss = MediaSourceStream::new(Box::new(media_source), Default::default());

        let mut format_reader = probe
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

        let mut decoder = codecs.make(codec_params, &Default::default()).unwrap();
        let mut frames = vec![];

        loop {
            match format_reader.next_packet() {
                Ok(packet) => {
                    let buffer = decoder.decode(&packet).unwrap();
                    frames.extend(copy_frames_from_buffer_ref(&buffer));
                }
                Err(err) => match err {
                    symphonia::core::errors::Error::IoError(err) => {
                        if err.kind() == std::io::ErrorKind::UnexpectedEof {
                            break;
                        }

                        panic!("{}", err);
                    }
                    _ => {
                        panic!("{}", err);
                    }
                },
            }
        }

        Self {
            frames,
            sample_rate,
        }
    }
}

fn copy_frames_from_buffer_ref(src: &AudioBufferRef) -> Vec<Frame> {
    match src {
        AudioBufferRef::U8(buf) => copy_frames_from_buffer(&buf),
        AudioBufferRef::U16(buf) => copy_frames_from_buffer(&buf),
        AudioBufferRef::U24(buf) => copy_frames_from_buffer(&buf),
        AudioBufferRef::U32(buf) => copy_frames_from_buffer(&buf),
        AudioBufferRef::S8(buf) => copy_frames_from_buffer(&buf),
        AudioBufferRef::S16(buf) => copy_frames_from_buffer(&buf),
        AudioBufferRef::S24(buf) => copy_frames_from_buffer(&buf),
        AudioBufferRef::S32(buf) => copy_frames_from_buffer(&buf),
        AudioBufferRef::F32(buf) => copy_frames_from_buffer(&buf),
        AudioBufferRef::F64(buf) => copy_frames_from_buffer(&buf),
    }
}

fn copy_frames_from_buffer<T>(src: &AudioBuffer<T>) -> Vec<Frame>
where
    f32: FromSample<T>,
    T: Sample,
{
    match src.spec().channels.count() {
        1 => src
            .chan(0)
            .iter()
            .map(|sample| Frame {
                left: f32::from_sample(*sample),
                right: f32::from_sample(*sample),
            })
            .collect(),
        2 => src
            .chan(0)
            .iter()
            .zip(src.chan(1).iter())
            .map(|(left, right)| Frame {
                left: f32::from_sample(*left),
                right: f32::from_sample(*right),
            })
            .collect(),
        _ => panic!("unsupported channel config"),
    }
}
