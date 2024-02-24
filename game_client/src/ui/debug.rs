use std::collections::VecDeque;
use std::time::Duration;

use game_core::counter::UpdateCounter;
use game_ui::reactive::Scope;
use game_ui::widgets::{Container, Plot, Text, Widget};
use glam::{UVec2, Vec2};

pub struct DebugUi {
    pub stats: Statistics,
    pub ups: Vec<Vec2>,
    pub fps: Vec<Vec2>,
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
        list.append(Text::new().text(format!("Update time")));
        list.append(Plot {
            size: UVec2::new(256, 128),
            points: self.ups,
        });
        list.append(Text::new().text(format!("Frame time")));
        list.append(Plot {
            size: UVec2::new(256, 128),
            points: self.fps,
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
    max: Duration,
    samples: VecDeque<Duration>,
}

impl FrametimeGraph {
    pub fn push(&mut self, sample: Duration) {
        if sample > self.max {
            self.max = sample;
        }

        if self.samples.len() > 300 {
            self.samples.pop_front();
        }

        self.samples.push_back(sample);
    }

    pub fn points(&self) -> Vec<Vec2> {
        self.samples
            .iter()
            .enumerate()
            .map(|(index, sample)| {
                let x = index as f32 / 300.0;
                let y = sample.as_nanos() as f32 / self.max.as_nanos() as f32;
                Vec2 { x, y }
            })
            .collect()
    }
}
