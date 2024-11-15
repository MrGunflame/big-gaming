use game_tracing::trace_span;
use rubato::{
    FftFixedInOut, Resampler as _, SincFixedIn, SincInterpolationParameters, SincInterpolationType,
    WindowFunction,
};

use crate::buffer::{Buf, BufMut};
use crate::sound::Frame;
use crate::sound_data::SoundData;

#[derive(Clone, Debug)]
pub enum Error {
    InputTooSmall(usize),
    OutputTooSmall(usize),
}

#[derive(Copy, Clone, Debug)]
pub(crate) struct ResampleOutput {
    /// The number of frames that were consumed from the source buffer in a [`resample`]
    /// operation.
    ///
    /// [`resample`]: Resampler::resample
    pub(crate) frames_read: usize,
    /// The number of frames that were written to the destination buffer in a [`resample`]
    /// operation.
    ///
    /// [`resample`]: Resampler::resample
    pub(crate) frames_written: usize,
}

pub struct Resampler {
    inner: SincFixedIn<f32>,
}

impl Resampler {
    pub fn new(src_sample_rate: u32, dst_sample_rate: u32) -> Self {
        let ratio = dst_sample_rate as f64 / src_sample_rate as f64;

        Self {
            inner: SincFixedIn::new(
                ratio,
                1.0,
                SincInterpolationParameters {
                    sinc_len: 256,
                    f_cutoff: 0.95,
                    oversampling_factor: 128,
                    interpolation: SincInterpolationType::Cubic,
                    window: WindowFunction::Blackman,
                },
                4096,
                2,
            )
            .unwrap(),
        }
    }

    pub fn resample<Src, Dst>(&mut self, src: Src, mut dst: Dst) -> Result<ResampleOutput, Error>
    where
        Src: Buf<Sample = f32>,
        Dst: BufMut<Sample = f32>,
    {
        if src.num_frames() < self.inner.input_frames_next() {
            return Err(Error::InputTooSmall(self.inner.input_frames_next()));
        }

        if dst.num_frames() < self.inner.output_frames_next() {
            return Err(Error::OutputTooSmall(self.inner.output_frames_next()));
        }

        let wave_in = &[src.channel(0).unwrap(), src.channel(1).unwrap()];
        let (left, right) = dst.channel_mut2(0, 1).unwrap();
        let wave_out = &mut [left, right];

        let (frames_read, frames_written) = self
            .inner
            .process_into_buffer(wave_in, wave_out, None)
            .unwrap();

        Ok(ResampleOutput {
            frames_read,
            frames_written,
        })
    }
}

pub fn resample(data: SoundData, sample_rate: u32) -> SoundData {
    let _span = trace_span!("resample").entered();

    let mut resampler =
        FftFixedInOut::<f32>::new(data.sample_rate as usize, sample_rate as usize, 1, 2).unwrap();

    let mut output_frames = Vec::new();

    let mut index = 0;
    while index < data.frames.len() {
        let input_len = resampler.input_frames_next();
        let output_len = resampler.output_frames_next();

        let left: Vec<f32> = data
            .frames
            .iter()
            .skip(index)
            .take(input_len)
            .map(|f| f.left)
            .collect();
        let right: Vec<f32> = data
            .frames
            .iter()
            .skip(index)
            .take(input_len)
            .map(|f| f.right)
            .collect();

        if left.len() < input_len || right.len() < input_len {
            break;
        }

        let mut output_left = vec![0.0; output_len];
        let mut output_right = vec![0.0; output_len];

        resampler
            .process_into_buffer(
                &[left, right],
                &mut [&mut output_left, &mut output_right],
                None,
            )
            .unwrap();

        output_frames.extend(
            output_left
                .into_iter()
                .zip(output_right.into_iter())
                .map(|(left, right)| Frame { left, right }),
        );

        index += input_len;
    }

    SoundData {
        frames: output_frames,
        sample_rate,
        volume: data.volume,
    }
}
