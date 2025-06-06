use std::sync::Arc;

use game_render::statistics::Statistics;

use crate::runtime::Context;
use crate::widgets::{Container, Text, Widget};

#[derive(Clone, Debug)]
pub struct GpuTimings {
    pub stats: Arc<Statistics>,
}

impl Widget for GpuTimings {
    fn mount(self, parent: &Context) -> Context {
        let root = Container::new().mount(parent);

        let timings = self.stats.gpu_timings.read();

        Text::new(format!("GPU Time: {:?}", timings.time)).mount(&root);

        for pass in &timings.passes {
            Text::new(format!("{}: {:?}", pass.name, pass.time)).mount(&root);
        }

        root
    }
}
