#![deny(unsafe_op_in_unsafe_fn)]

use bevy_app::{App, Plugin};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{default_host, Host};

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        let host = default_host();
        let device = host
            .default_output_device()
            .expect("no default output device");

        let config = device
            .supported_output_configs()
            .unwrap()
            .next()
            .unwrap()
            .with_max_sample_rate()
            .config();

        dbg!(&config);

        let sample_rate = config.sample_rate.0 as f32;
        let channels = config.channels as usize;

        // Produce a sinusoid of maximum amplitude.
        let mut sample_clock = 0f32;
        let mut next_value = move || {
            sample_clock = (sample_clock + 1.0) % sample_rate;
            (sample_clock * 440.0 * 2.0 * std::f32::consts::PI / sample_rate).sin()
        };

        let stream = device
            .build_output_stream(
                &config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    write_data(data, channels, &mut next_value);
                },
                move |err| {
                    panic!("{}", err);
                },
                None,
            )
            .unwrap();
        stream.play().unwrap();

        loop {}
    }
}

fn write_data(output: &mut [f32], channels: usize, next_sample: &mut dyn FnMut() -> f32) {
    for frame in output.chunks_mut(channels) {
        let val = next_sample();
        for sample in frame.iter_mut() {
            *sample = val;
        }
    }
}
