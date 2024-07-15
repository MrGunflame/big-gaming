use std::sync::Arc;

use game_common::module::{Dependency, Module, ModuleId, ModuleIdExt, Version};
use game_ui::reactive::Context;
use game_ui::style::{
    Background, BorderRadius, Bounds, Direction, Growth, Justify, Padding, Size, SizeVec2, Style,
};
use game_ui::widgets::{Button, Callback, Container, Input, Text, Widget};
use image::Rgba;
use parking_lot::Mutex;

use crate::state::capabilities::Capabilities;
use crate::state::module::{EditorModule, Modules};

const BACKGROUND_COLOR: Background = Background::Color(Rgba([0x35, 0x35, 0x35, 0xFF]));

pub struct CreateModule {
    pub modules: Modules,
}

impl Widget for CreateModule {
    fn mount<T>(self, parent: &Context<T>) -> Context<()> {
        EditModule {
            modules: self.modules,
            id: None,
        }
        .mount(parent)
    }
}

#[derive(Clone, Debug)]
pub struct EditModule {
    pub modules: Modules,
    pub id: Option<ModuleId>,
}

impl Widget for EditModule {
    fn mount<T>(self, parent: &Context<T>) -> Context<()> {
        let fields = if let Some(id) = self.id {
            let module = self.modules.get(id).unwrap();

            Fields {
                id: module.module.id,
                name: module.module.name,
                version: module.module.version,
                dependencies: module.module.dependencies,
            }
        } else {
            Fields {
                id: ModuleId::random(),
                name: String::new(),
                version: Version,
                dependencies: Vec::new(),
            }
        };

        let fields = Arc::new(Mutex::new(fields));

        let style = Style {
            justify: Justify::SpaceBetween,
            growth: Growth::splat(1.0),
            background: BACKGROUND_COLOR,
            padding: Padding::splat(Size::Pixels(5)),
            ..Default::default()
        };

        let root = Container::new().style(style).mount(parent);

        let table = Container::new()
            .style(Style {
                direction: Direction::Column,
                ..Default::default()
            })
            .mount(&root);

        let key_col = Container::new().mount(&table);
        let val_col = Container::new().mount(&table);

        for key in ["ID", "Name"] {
            Text::new(key).mount(&key_col);
        }

        Text::new(fields.lock().id.to_string()).mount(&val_col);

        let style = Style {
            bounds: Bounds {
                min: SizeVec2::splat(Size::Pixels(50)),
                ..Default::default()
            },
            background: Background::GRAY,
            padding: Padding::splat(Size::Pixels(2)),
            border_radius: BorderRadius::splat(Size::Pixels(2)),
            ..Default::default()
        };

        Input::new()
            .style(style)
            .on_change({
                let fields = fields.clone();
                move |s| fields.lock().name = s
            })
            .value(fields.lock().name.clone())
            .mount(&val_col);

        let bottom = Container::new()
            .style(Style {
                direction: Direction::Column,
                justify: Justify::Center,
                growth: Growth::x(1.0),
                ..Default::default()
            })
            .mount(&root);

        let on_create = on_create(self.modules, fields);

        let button = Button::new().on_click(on_create).mount(&bottom);
        Text::new("Ok").mount(&button);

        root
    }
}

fn on_create(modules: Modules, fields: Arc<Mutex<Fields>>) -> Callback<()> {
    Callback::from(move |()| {
        let fields = fields.lock();

        let module = EditorModule {
            module: Module {
                id: fields.id,
                name: fields.name.clone(),
                version: fields.version,
                dependencies: fields.dependencies.clone(),
            },
            path: None,
            capabilities: Capabilities::READ | Capabilities::WRITE,
        };

        modules.insert(module);

        // ctx.window.close();
    })
}

#[derive(Clone, Debug)]
struct Fields {
    id: ModuleId,
    name: String,
    version: Version,
    dependencies: Vec<Dependency>,
}
