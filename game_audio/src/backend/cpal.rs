use std::sync::mpsc;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{default_host, SampleRate};

use crate::channel::Receiver;
use crate::sound::Frame;

use super::Backend;

#[derive(Debug)]
pub struct CpalBackend {
    tx: mpsc::Sender<Receiver>,
}

impl CpalBackend {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel::<Receiver>();

        std::thread::spawn(move || {
            let host = default_host();
            let device = host
                .default_output_device()
                .expect("no default output device");

            let mut config = device
                .supported_output_configs()
                .unwrap()
                .next()
                .unwrap()
                .with_max_sample_rate()
                .config();

            config.sample_rate = SampleRate(48_000);
            config.channels = 2;

            let channels = config.channels as usize;

            let mut streams = vec![];

            while let Ok(mut buf) = rx.recv() {
                let stream = device
                    .build_output_stream(
                        &config,
                        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                            write_data(data, channels, &mut buf);
                        },
                        |err| {
                            panic!("{}", err);
                        },
                        None,
                    )
                    .unwrap();

                stream.play().unwrap();

                // Keep the stream alive until the backend itself is dropped.
                streams.push(stream);
            }

            drop(streams);
        });

        Self { tx }
    }
}

impl Backend for CpalBackend {
    fn create_output_stream(&mut self, rx: Receiver) {
        self.tx.send(rx);
    }
}

fn write_data(output: &mut [f32], channels: usize, rx: &mut Receiver) {
    for f in output.chunks_exact_mut(channels) {
        let frame = match rx.recv() {
            Some(frame) => frame,
            None => {
                //tracing::error!("no data");
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
