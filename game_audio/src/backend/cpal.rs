use cpal::default_host;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use crate::sound::{Frame, Receiver};

#[derive(Debug)]
pub(crate) struct CpalBackend {}

impl CpalBackend {
    pub fn new(rx: Receiver) -> Self {
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

        let channels = config.channels as usize;

        let stream = device
            .build_output_stream(
                &config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    let frame = rx.pop().unwrap_or(Frame::EQUILIBRIUM);
                    write_data(data, channels, frame);
                },
                |err| {
                    panic!("{}", err);
                },
                None,
            )
            .unwrap();

        stream.play().unwrap();

        Self {}
    }
}

fn write_data(output: &mut [f32], channels: usize, frame: Frame) {
    for f in output.chunks_exact_mut(channels) {
        match channels {
            1 => {
                f[0] = (frame.left) + (frame.right) / 2.0;
            }
            2 => {
                f[0] = frame.left;
                f[1] = frame.right;
            }
            // We only support mono/stereo for now.
            _ => panic!("invalid channel config: {}", channels),
        }
    }
}
