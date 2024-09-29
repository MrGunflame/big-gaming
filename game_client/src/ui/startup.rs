use game_ui::reactive::Context;
use game_ui::widgets::{Container, Text, Widget};

pub struct Startup {}

impl Widget for Startup {
    fn mount<T>(self, parent: &Context<T>) -> Context<()> {
        let root = Container::new().mount(parent);
        Text::new("Loading").mount(&root);
        root
    }
}
