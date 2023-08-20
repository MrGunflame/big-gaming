use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{default_host, Stream};

use crate::sound::{Frame, Receiver};

#[derive(Debug)]
pub(crate) struct CpalBackend {}

impl CpalBackend {
    pub fn new(mut rx: Receiver) -> Self {
        std::thread::spawn(move || {
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

            dbg!(&config.sample_rate);

            let stream = device
                .build_output_stream(
                    &config,
                    move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                        write_data(data, channels, &mut rx);
                    },
                    |err| {
                        panic!("{}", err);
                    },
                    None,
                )
                .unwrap();

            stream.play().unwrap();

            loop {
                std::thread::yield_now();
            }
        });

        Self {}
    }
}

fn write_data(output: &mut [f32], channels: usize, rx: &mut Receiver) {
    for f in output.chunks_exact_mut(channels) {
        let frame = match rx.pop() {
            Some(frame) => frame,
            None => {
                tracing::error!("no data");
                Frame::EQUILIBRIUM
            }
        };

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
