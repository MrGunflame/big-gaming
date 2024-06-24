use std::collections::VecDeque;
use std::fmt::{self, Display, Formatter};
use std::time::Duration;

use game_core::counter::UpdateCounter;
use game_ui::reactive::Context;
use game_ui::widgets::{Container, Plot, Text, Widget};
use glam::{UVec2, Vec2};

pub struct DebugUi {
    pub stats: Statistics,
    pub(super) ups: FrametimeGraph,
    pub(super) fps: FrametimeGraph,
    pub(super) rtt: FrametimeGraph,
}

impl Widget for DebugUi {
    fn mount<T>(self, parent: &Context<T>) -> Context<()> {
        let list = Container::new().mount(parent);

        Text::new(format!(
            "UPS: {:.2} FPS: {:.2}",
            self.stats.ups.ups(),
            self.stats.fps.ups()
        ))
        .mount(&list);
        Text::new(format!("Entities: {}", self.stats.entities)).mount(&list);
        Text::new(format!(
            "Unacked predicted inputs: {}",
            self.stats.net_input_buffer_len
        ))
        .mount(&list);
        let ups = self.ups.stats();
        Text::new(format!(
            "Update time (min={} max={} mean={} stddev={})",
            DurationFormat(ups.min),
            DurationFormat(ups.max),
            DurationFormat(ups.mean),
            DurationFormat(ups.stddev)
        ))
        .mount(&list);
        Plot {
            size: UVec2::new(256, 128),
            points: self.ups.points(),
        }
        .mount(&list);
        let fps = self.fps.stats();
        Text::new(format!(
            "Frame time (min={} max={} mean={} stddev={})",
            DurationFormat(fps.min),
            DurationFormat(fps.max),
            DurationFormat(fps.mean),
            DurationFormat(fps.stddev)
        ))
        .mount(&list);
        Plot {
            size: UVec2::new(256, 128),
            points: self.fps.points(),
        }
        .mount(&list);

        let rtt = self.rtt.stats();
        Text::new(format!(
            "RTT (min={} max={} mean={} stddev={})",
            DurationFormat(rtt.min),
            DurationFormat(rtt.max),
            DurationFormat(rtt.mean),
            DurationFormat(rtt.stddev),
        ))
        .mount(&list);
        Plot {
            size: UVec2::new(256, 128),
            points: self.rtt.points(),
        }
        .mount(&list);

        list
    }
}

#[derive(Clone, Debug)]
pub struct Statistics {
    pub ups: UpdateCounter,
    pub fps: UpdateCounter,
    pub entities: u64,
    pub net_input_buffer_len: u64,
    pub rtt: Duration,
}

#[derive(Clone, Debug, Default)]
pub(super) struct FrametimeGraph {
    samples: VecDeque<Duration>,
}

impl FrametimeGraph {
    pub fn push(&mut self, sample: Duration) {
        if self.samples.len() > 300 {
            self.samples.pop_front();
        }

        self.samples.push_back(sample);
    }

    pub fn stats(&self) -> GraphStats {
        let mut min = Duration::MAX;
        let mut max = Duration::ZERO;

        let mut sum = Duration::ZERO;
        for sample in &self.samples {
            min = Duration::min(min, *sample);
            max = Duration::max(max, *sample);
            sum += *sample;
        }

        let mean = sum / self.samples.len() as u32;

        let mut values = 0;
        for sample in &self.samples {
            values += (sample.as_nanos().saturating_sub(mean.as_nanos())).pow(2) as u64;
        }
        let stddev =
            Duration::from_nanos(f64::sqrt((values / self.samples.len() as u64) as f64) as u64);

        GraphStats {
            min,
            max,
            mean,
            stddev,
        }
    }

    pub fn points(&self) -> Vec<Vec2> {
        let stats = self.stats();

        self.samples
            .iter()
            .enumerate()
            .map(|(index, sample)| {
                let x = index as f32 / 300.0;
                let y = sample.as_nanos() as f32 / stats.max.as_nanos() as f32;
                Vec2 { x, y }
            })
            .collect()
    }
}

pub struct GraphStats {
    pub min: Duration,
    pub max: Duration,
    pub mean: Duration,
    pub stddev: Duration,
}

struct DurationFormat(Duration);

impl Display for DurationFormat {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let nanos = self.0.as_nanos();
        let millis = nanos / 1_000_000;
        let micros = (nanos - millis * 1_000_000) / 1_000;

        write!(f, "{}.{}ms", millis, micros)
    }
}
