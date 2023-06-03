use game_common::module::ModuleId;
use game_ui::reactive::Scope;
use game_ui::render::style::{
    Background, Bounds, Direction, Growth, Justify, Size, SizeVec2, Style,
};
use game_ui::{component, view};

use game_ui::widgets::*;

#[component]
pub fn CreateModule(cx: &Scope) -> Scope {
    let root = view! {
        cx,
        <Container style={Style { justify: Justify::SpaceBetween, growth: Growth::splat(1.0), ..Default::default() }}>
        </Container>
    };

    let id = ModuleId::random();

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
        <Text text={id.to_string().into()}>
        </Text>
    };

    let style = Style {
        bounds: Bounds {
            min: SizeVec2::splat(Size::Pixels(50.0)),
            ..Default::default()
        },
        background: Background::BLACK,
        ..Default::default()
    };

    view! {
        val_col,
        <Input value={String::new()} style={style}>
        </Input>
    };

    let bottom = view! {
        root,
        <Container style={Style { direction: Direction::Column, justify: Justify::Center, growth: Growth::x(1.0), ..Default::default() }}>
        </Container>
    };

    view! {
        bottom,
        <Button style={Style::default()} on_click={on_create().into()}>
            <Text text={"OK".into()}>
            </Text>
        </Button>
    };

    cx.clone()
}

fn on_create() -> Box<dyn Fn() + Send + Sync + 'static> {
    Box::new(move || {})
}

fn on_cancel() -> Box<dyn Fn() + Send + Sync + 'static> {
    Box::new(move || {})
}
