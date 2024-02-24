use std::fmt::{self, Debug, Formatter};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{
    default_host, BufferSize, Device, DevicesError, Host, SampleRate, StreamConfig,
    SupportedStreamConfigsError,
};
use thiserror::Error;

use crate::channel::Receiver;
use crate::sound::Frame;

use super::Backend;

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot enumerate devices: {0}")]
    Devices(#[from] DevicesError),
    #[error("cannot select device")]
    NoDevice,
    #[error("cannot enumerate stream configs: {0}")]
    Config(#[from] SupportedStreamConfigsError),
    #[error("cannot select stream config")]
    NoConfig,
}

pub struct CpalBackend {
    device: Device,
    streams: Vec<cpal::Stream>,
    config: StreamConfig,
}

impl CpalBackend {
    pub fn new() -> Result<Self, Error> {
        let sample_rate = SampleRate(48_000);
        let channels = 2;

        let (device, config) = new_inner(sample_rate, channels)?;

        Ok(Self {
            device,
            streams: Vec::new(),
            config,
        })
    }
}

impl Debug for CpalBackend {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("CpalBackend").finish_non_exhaustive()
    }
}

impl Backend for CpalBackend {
    fn create_output_stream(&mut self, mut rx: Receiver) {
        let channels = self.config.channels as usize;

        let stream = self
            .device
            .build_output_stream(
                &self.config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    write_data(data, channels, &mut rx)
                },
                |err| {
                    panic!("output stream error: {}", err);
                },
                None,
            )
            .unwrap();
        stream.play().unwrap();
        self.streams.push(stream);
    }
}

fn new_inner(sample_rate: SampleRate, channels: u16) -> Result<(Device, StreamConfig), Error> {
    let host = default_host();
    let device = select_output_device(&host)?;

    for config in device.supported_output_configs()? {
        if config.min_sample_rate() < sample_rate
            && config.max_sample_rate() > sample_rate
            && config.channels() == channels
        {
            return Ok((
                device,
                StreamConfig {
                    channels,
                    sample_rate,
                    buffer_size: BufferSize::Default,
                },
            ));
        }
    }

    Err(Error::NoConfig)
}

fn select_output_device(host: &Host) -> Result<Device, Error> {
    match host.default_output_device() {
        Some(device) => Ok(device),
        None => {
            let mut devices = host.output_devices()?;
            devices.next().ok_or(Error::NoDevice)
        }
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
