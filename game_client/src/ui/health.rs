use game_ui::reactive::Context;
use game_ui::style::{Bounds, Size, SizeVec2, Style};
use game_ui::widgets::{Container, Text, Widget};

use crate::components::base::Health;

pub struct HealthUi {
    pub health: Health,
}

impl Widget for HealthUi {
    fn mount<T>(self, parent: &Context<T>) -> Context<()> {
        let root = Container::new()
            .style(Style {
                bounds: Bounds::from_min(SizeVec2::splat(Size::ZERO)),
                ..Default::default()
            })
            .mount(parent);

        let label = format!("{}/{}", self.health.value, self.health.max);

        Text::new(label).mount(&root);
        root
    }
}
