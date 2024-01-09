use game_ui::reactive::Scope;
use game_ui::widgets::{Container, Text, Widget};

pub struct DebugUi {
    pub stats: Statistics,
}

impl Widget for DebugUi {
    fn build(self, cx: &Scope) -> Scope {
        let list = cx.append(Container::new());

        list.append(Text::new().text(format!(
            "UPS: {:.2} FPS: {:.2}",
            self.stats.ups, self.stats.fps
        )));
        list.append(Text::new().text(format!("Entities: {}", self.stats.entities)));
        list.append(Text::new().text(format!(
            "Unacked predicted inputs: {}",
            self.stats.net_input_buffer_len
        )));

        list
    }
}

#[derive(Clone, Debug)]
pub struct Statistics {
    pub ups: f32,
    pub fps: f32,
    pub entities: u64,
    pub net_input_buffer_len: u64,
}
