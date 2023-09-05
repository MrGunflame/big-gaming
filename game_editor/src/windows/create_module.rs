use game_common::module::{Dependency, Module, ModuleId, Version};
use game_input::mouse::MouseButtonInput;
use game_ui::events::Context;
use game_ui::reactive::{create_signal, ReadSignal, Scope};
use game_ui::render::style::{
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
        let (id, set_id) = create_signal(cx, ModuleId::random());
        let (name, set_name) = create_signal(cx, String::new());
        let (version, set_version) = create_signal(cx, Version);
        let (dependencies, set_dependencies) = create_signal(cx, Vec::new());

        let style = Style {
            justify: Justify::SpaceBetween,
            growth: Growth::splat(1.0),
            background: BACKGROUND_COLOR,
            padding: Padding::splat(Size::Pixels(5.0)),
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
            key_col.append(Text::new().text(key));
        }

        val_col.append(Text::new().text(id.get_untracked()));

        let style = Style {
            bounds: Bounds {
                min: SizeVec2::splat(Size::Pixels(50.0)),
                ..Default::default()
            },
            background: Background::GRAY,
            padding: Padding::splat(Size::Pixels(2.0)),
            border_radius: BorderRadius::splat(Size::Pixels(2.0)),
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
        button.append(Text::new().text("OK"));

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
