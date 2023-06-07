use game_common::module::{Dependency, Module, ModuleId, Version};
use game_ui::reactive::{create_signal, ReadSignal, Scope};
use game_ui::render::style::{
    Background, BorderRadius, Bounds, Direction, Growth, Justify, Padding, Size, SizeVec2, Style,
};
use game_ui::{component, view};

use game_ui::widgets::*;
use image::Rgba;

use crate::state::capabilities::Capabilities;
use crate::state::module::{EditorModule, Modules};

const BACKGROUND_COLOR: Background = Background::Color(Rgba([0x35, 0x35, 0x35, 0xFF]));

#[component]
pub fn CreateModule(cx: &Scope, modules: Modules) -> Scope {
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

    let root = view! {
        cx,
        <Container style={style}>
        </Container>
    };

    let table = view! {
        root,
        <Container style={Style { direction: Direction::Column, ..Default::default() }}>
        </Container>
    };

    let key_col = view! {
        table,
        <Container style={Style::default()}>
        </Container>
    };

    for key in ["ID", "Name"] {
        view! {
            key_col,
            <Text text={key.into()}>
            </Text>
        };
    }

    let val_col = view! {
        table,
        <Container style={Style::default()}>
        </Container>
    };

    view! {
        val_col,
        <Text text={id.get_untracked().to_string().into()}>
        </Text>
    };

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

    view! {
        val_col,
        <Input value={String::new()} style={style} on_change={Box::new(move |s|{ set_name.update(|val| *val = s)})}>
        </Input>
    };

    let bottom = view! {
        root,
        <Container style={Style { direction: Direction::Column, justify: Justify::Center, growth: Growth::x(1.0), ..Default::default() }}>
        </Container>
    };

    let on_create = on_create(
        modules,
        Fields {
            id,
            name,
            version,
            dependencies,
        },
    );

    view! {
        bottom,
        <Button style={Style::default()} on_click={on_create.into()}>
            <Text text={"OK".into()}>
            </Text>
        </Button>
    };

    cx.clone()
}

fn on_create(modules: Modules, fields: Fields) -> Box<dyn Fn() + Send + Sync + 'static> {
    Box::new(move || {
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
    })
}

#[derive(Debug)]
struct Fields {
    id: ReadSignal<ModuleId>,
    name: ReadSignal<String>,
    version: ReadSignal<Version>,
    dependencies: ReadSignal<Vec<Dependency>>,
}
