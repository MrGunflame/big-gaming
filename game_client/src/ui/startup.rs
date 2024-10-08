use game_ui::runtime::Context;
use game_ui::widgets::{Container, Text, Widget};

pub struct Startup {}

impl Widget for Startup {
    fn mount(self, parent: &Context) -> Context {
        let root = Container::new().mount(parent);
        Text::new("Loading").mount(&root);
        root
    }
}
