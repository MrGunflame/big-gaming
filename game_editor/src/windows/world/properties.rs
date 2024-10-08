use std::sync::mpsc;

use game_render::options::ShadingMode;
use game_ui::runtime::Context;
use game_ui::widgets::{Button, Container, Text, Widget};

use super::Event;

#[derive(Clone, Debug)]
pub struct Properties {
    pub writer: mpsc::Sender<Event>,
}

impl Widget for Properties {
    fn mount(self, parent: &Context) -> Context {
        let root = Container::new().mount(parent);

        let mut shading_mode = ShadingMode::Full;
        let button = Button::new()
            .on_click(move |()| {
                let new_mode = match shading_mode {
                    ShadingMode::Full => ShadingMode::Albedo,
                    ShadingMode::Albedo => ShadingMode::Normal,
                    ShadingMode::Normal => ShadingMode::Tangent,
                    ShadingMode::Tangent => ShadingMode::Full,
                };

                shading_mode = new_mode;
                self.writer
                    .send(Event::SetShadingMode(shading_mode))
                    .unwrap();
            })
            .mount(&root);
        Text::new("Shading Mode").mount(&button);

        root
    }
}
