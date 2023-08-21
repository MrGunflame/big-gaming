use crate::sound::Frame;
use crate::sound_data::SoundData;

pub fn resample(data: SoundData, sample_rate: u32) -> SoundData {
    let duration = data.frames.len() / data.sample_rate as usize;

    let mut output = SoundData {
        frames: vec![Frame::EQUILIBRIUM; duration * sample_rate as usize],
        sample_rate,
        volume: data.volume,
    };

    for frame in &mut output.frames {}

    output
}
