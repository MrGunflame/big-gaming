use std::collections::VecDeque;
use std::fmt::{self, Display, Formatter};
use std::time::Duration;

use game_common::components::{RigidBody, Transform};
use game_core::counter::UpdateCounter;
use game_ui::reactive::Context;
use game_ui::widgets::{Container, Plot, Text, Widget};
use glam::{UVec2, Vec2, Vec3};

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

        if let Some(transform) = self.stats.player_info.transform {
            Text::new(format!(
                "Translation: X={:.2} Y={:.2} Z={:.2}",
                transform.translation.x, transform.translation.y, transform.translation.z
            ))
            .mount(&list);
            let direction = transform.rotation * -Vec3::Z;
            Text::new(format!(
                "Rotation: X={:.2} Y={:.2} Z={:.2} W={:.2} (facing X={:.2} Y={:.2} Z={:.2})",
                transform.rotation.x,
                transform.rotation.y,
                transform.rotation.z,
                transform.rotation.w,
                direction.x,
                direction.y,
                direction.z,
            ))
            .mount(&list);
        }

        if let Some(rigid_body) = self.stats.player_info.rigid_body {
            Text::new(format!(
                "Linear: x={:.2} Y={:.2} Z={:.2}",
                rigid_body.linvel.x, rigid_body.linvel.y, rigid_body.linvel.z,
            ))
            .mount(&list);
            Text::new(format!(
                "Angular: x={:.2} Y={:.2} Z={:.2}",
                rigid_body.angvel.x, rigid_body.angvel.y, rigid_body.angvel.z,
            ))
            .mount(&list);
        }

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
    pub player_info: PlayerInfo,
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
            sum = sum.saturating_add(*sample);
        }

        let mean = sum
            .checked_div(self.samples.len() as u32)
            .unwrap_or_default();

        let mut values: u64 = 0;
        for sample in &self.samples {
            values = values.saturating_add(
                (sample.as_nanos().saturating_sub(mean.as_nanos()))
                    .saturating_pow(2)
                    .try_into()
                    .unwrap_or(u64::MAX),
            );
        }
        let stddev = Duration::from_nanos(f64::sqrt(
            (values
                .checked_div(self.samples.len() as u64)
                .unwrap_or_default()) as f64,
        ) as u64);

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

#[derive(Clone, Debug, Default)]
pub(crate) struct PlayerInfo {
    pub transform: Option<Transform>,
    pub rigid_body: Option<RigidBody>,
}
