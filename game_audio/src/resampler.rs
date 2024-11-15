use std::fmt::{self, Debug, Formatter};

use game_tracing::trace_span;
use rubato::{
    Resampler as _, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction,
};

use crate::buffer::{Buf, BufMut};

#[derive(Clone, Debug)]
pub(crate) enum Error {
    /// Returned when the input buffer given to [`resample`] is too small.
    ///
    /// The value represents the minimum required input buffer size in frames.
    ///
    /// [`resample`]: Resampler::resample
    InputTooSmall(usize),
    /// Returned when the output buffer given to [`resample`] is too small.
    ///
    /// The value represents the minimum required output buffer size in frames.
    ///
    /// [`resample`]: Resampler::resample
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

/// A resampler for the conversation of sample rates.
pub(crate) struct Resampler {
    inner: SincFixedIn<f32>,
}

impl Resampler {
    /// Creates a new `Resampler` that resamples from `src_sample_rate` to `dst_sample_rate`.
    pub(crate) fn new(src_sample_rate: u32, dst_sample_rate: u32) -> Self {
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

    /// Reads and resamples frames from `src` and writes them into `dst`. Returns the number of
    /// frames read and written on success.
    ///
    /// # Errors
    ///
    /// Returns an appropriate [`Error`] if either the `src` or `dst` buffers are too small to
    /// complete the resample operation.
    pub(crate) fn resample<Src, Dst>(
        &mut self,
        src: Src,
        mut dst: Dst,
    ) -> Result<ResampleOutput, Error>
    where
        Src: Buf<Sample = f32>,
        Dst: BufMut<Sample = f32>,
    {
        let _span = trace_span!("Resampler::resample").entered();

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

impl Debug for Resampler {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Resampler").finish_non_exhaustive()
    }
}
