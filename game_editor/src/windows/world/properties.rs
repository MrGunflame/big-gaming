use std::sync::mpsc;

use game_render::options::ShadingMode;
use game_ui::runtime::Context;
use game_ui::style::{Color, Direction, Style};
use game_ui::widgets::{Button, Container, Svg, SvgData, SvgStyle, Widget};

use super::Event;

const ICON_SHADING_MODE_ALBEDO: &[u8] =
    include_bytes!("../../../../assets/fonts/FontAwesome/svgs/solid/1.svg");
const ICON_SHADING_MODE_NORMAL: &[u8] =
    include_bytes!("../../../../assets/fonts/FontAwesome/svgs/solid/2.svg");
const ICON_SHADING_MODE_TANGENT: &[u8] =
    include_bytes!("../../../../assets/fonts/FontAwesome/svgs/solid/3.svg");
const ICON_SHADING_MODE_FULL: &[u8] =
    include_bytes!("../../../../assets/fonts/FontAwesome/svgs/solid/4.svg");

const DEFAULT_SHADING_MODE: ShadingMode = ShadingMode::Albedo;

#[derive(Clone, Debug)]
pub struct Properties {
    pub writer: mpsc::Sender<Event>,
}

impl Widget for Properties {
    fn mount(self, parent: &Context) -> Context {
        let root = Container::new()
            .style(Style {
                direction: Direction::Column,
                ..Default::default()
            })
            .mount(parent);

        ShadingModeSelection {
            writer: self.writer.clone(),
        }
        .mount(&root);

        root
    }
}

#[derive(Debug)]
struct ShadingModeSelection {
    writer: mpsc::Sender<Event>,
}

impl Widget for ShadingModeSelection {
    fn mount(self, parent: &Context) -> Context {
        // Set the shading mode when the widget is first created.
        // This ensures we actually are in the default shading mode.
        self.writer
            .send(Event::SetShadingMode(DEFAULT_SHADING_MODE))
            .unwrap();

        let root = Container::new()
            .style(Style {
                direction: Direction::Column,
                ..Default::default()
            })
            .mount(parent);

        for (mode, icon) in [
            (ShadingMode::Albedo, ICON_SHADING_MODE_ALBEDO),
            (ShadingMode::Normal, ICON_SHADING_MODE_NORMAL),
            (ShadingMode::Tangent, ICON_SHADING_MODE_TANGENT),
            (ShadingMode::Full, ICON_SHADING_MODE_FULL),
        ] {
            let writer = self.writer.clone();

            let button = Button::new()
                .on_click(move |()| {
                    writer.send(Event::SetShadingMode(mode)).unwrap();
                })
                .mount(&root);
            Svg::new(SvgData::from_bytes(icon).unwrap(), 32, 32)
                .style(SvgStyle {
                    color: Some(Color::WHITE),
                })
                .mount(&button);
        }

        root
    }
}
