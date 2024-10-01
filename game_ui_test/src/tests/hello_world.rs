use std::convert::Infallible;

use game_ui::runtime_v2::{Context, View, Widget};
use game_ui::widgets::Text;

pub struct HelloWorld;

impl Widget for HelloWorld {
    type Message = Infallible;

    fn render(&self, _ctx: &Context<Self>) -> View {
        Text::new("Hello World!").size(32.0).into()
    }
}
