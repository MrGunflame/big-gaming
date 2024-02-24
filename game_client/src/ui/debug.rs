use std::collections::VecDeque;
use std::fmt::{self, Display, Formatter};
use std::time::Duration;

use game_core::counter::UpdateCounter;
use game_ui::reactive::Scope;
use game_ui::widgets::{Container, Plot, Text, Widget};
use glam::{UVec2, Vec2};

pub struct DebugUi {
    pub stats: Statistics,
    pub(super) ups: FrametimeGraph,
    pub(super) fps: FrametimeGraph,
}

impl Widget for DebugUi {
    fn build(self, cx: &Scope) -> Scope {
        let list = cx.append(Container::new());

        list.append(Text::new().text(format!(
            "UPS: {:.2} FPS: {:.2}",
            self.stats.ups.ups(),
            self.stats.fps.ups()
        )));
        list.append(Text::new().text(format!("Entities: {}", self.stats.entities)));
        list.append(Text::new().text(format!(
            "Unacked predicted inputs: {}",
            self.stats.net_input_buffer_len
        )));
        let ups = self.ups.stats();
        list.append(Text::new().text(format!(
            "Update time (min={} max={} mean={} stddev={})",
            DurationFormat(ups.min),
            DurationFormat(ups.max),
            DurationFormat(ups.mean),
            DurationFormat(ups.stddev)
        )));
        list.append(Plot {
            size: UVec2::new(256, 128),
            points: self.ups.points(),
        });
        let fps = self.fps.stats();
        list.append(Text::new().text(format!(
            "Frame time (min={} max={} mean={} stddev={})",
            DurationFormat(fps.min),
            DurationFormat(fps.max),
            DurationFormat(fps.mean),
            DurationFormat(fps.stddev)
        )));
        list.append(Plot {
            size: UVec2::new(256, 128),
            points: self.fps.points(),
        });

        list
    }
}

#[derive(Clone, Debug)]
pub struct Statistics {
    pub ups: UpdateCounter,
    pub fps: UpdateCounter,
    pub entities: u64,
    pub net_input_buffer_len: u64,
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
            values += (sample.as_nanos() - mean.as_nanos()).pow(2) as u64;
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
