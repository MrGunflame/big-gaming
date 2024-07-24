use game_tracing::trace_span;
use rubato::{FftFixedInOut, Resampler};

use crate::sound::Frame;
use crate::sound_data::SoundData;

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
