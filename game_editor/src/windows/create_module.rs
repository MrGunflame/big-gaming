use game_common::module::{Dependency, Module, ModuleId, ModuleIdExt, Version};
use game_input::mouse::MouseButtonInput;
use game_ui::events::Context;
use game_ui::reactive::{ReadSignal, Scope};
use game_ui::style::{
    Background, BorderRadius, Bounds, Direction, Growth, Justify, Padding, Size, SizeVec2, Style,
};
use game_ui::widgets::{Button, Callback, Container, Input, Text, Widget};
use image::Rgba;

use crate::state::capabilities::Capabilities;
use crate::state::module::{EditorModule, Modules};

const BACKGROUND_COLOR: Background = Background::Color(Rgba([0x35, 0x35, 0x35, 0xFF]));

pub struct CreateModule {
    pub modules: Modules,
}

impl Widget for CreateModule {
    fn build(self, cx: &Scope) -> Scope {
        let (id, set_id) = cx.create_signal(ModuleId::random());
        let (name, set_name) = cx.create_signal(String::new());
        let (version, set_version) = cx.create_signal(Version);
        let (dependencies, set_dependencies) = cx.create_signal(Vec::new());

        let style = Style {
            justify: Justify::SpaceBetween,
            growth: Growth::splat(1.0),
            background: BACKGROUND_COLOR,
            padding: Padding::splat(Size::Pixels(5)),
            ..Default::default()
        };

        let root = cx.append(Container::new().style(style));

        let table = root.append(Container::new().style(Style {
            direction: Direction::Column,
            ..Default::default()
        }));

        let key_col = table.append(Container::new());
        let val_col = table.append(Container::new());

        for key in ["ID", "Name"] {
            key_col.append(Text::new().text(key.to_owned()));
        }

        val_col.append(Text::new().text(id.get_untracked().to_string()));

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

        val_col.append(
            Input::new()
                .style(style)
                .on_change(move |s| set_name.update(|val| *val = s)),
        );

        let bottom = root.append(Container::new().style(Style {
            direction: Direction::Column,
            justify: Justify::Center,
            growth: Growth::x(1.0),
            ..Default::default()
        }));

        let on_create = on_create(
            self.modules,
            Fields {
                id,
                name,
                version,
                dependencies,
            },
        );

        let button = bottom.append(Button::new().on_click(on_create));
        button.append(Text::new().text("OK".to_owned()));

        root
    }
}

fn on_create(modules: Modules, fields: Fields) -> Callback<Context<MouseButtonInput>> {
    Callback::from(move |ctx: Context<MouseButtonInput>| {
        let module = EditorModule {
            module: Module {
                id: fields.id.get_untracked(),
                name: fields.name.get_untracked(),
                version: fields.version.get_untracked(),
                dependencies: fields.dependencies.get_untracked(),
            },
            path: None,
            capabilities: Capabilities::READ | Capabilities::WRITE,
        };

        modules.insert(module);

        ctx.window.close();
    })
}

#[derive(Debug)]
struct Fields {
    id: ReadSignal<ModuleId>,
    name: ReadSignal<String>,
    version: ReadSignal<Version>,
    dependencies: ReadSignal<Vec<Dependency>>,
}
