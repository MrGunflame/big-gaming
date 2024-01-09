use game_ui::reactive::Scope;
use game_ui::style::{Bounds, Size, SizeVec2, Style};
use game_ui::widgets::{Container, Text, Widget};

use crate::components::base::Health;

pub struct HealthUi {
    pub health: Health,
}

impl Widget for HealthUi {
    fn build(self, cx: &Scope) -> Scope {
        let root = cx.append(Container::new().style(Style {
            bounds: Bounds::from_min(SizeVec2::splat(Size::ZERO)),
            ..Default::default()
        }));

        let label = format!("{}/{}", self.health.value, self.health.max);

        root.append(Text::new().text(label));
        root
    }
}
