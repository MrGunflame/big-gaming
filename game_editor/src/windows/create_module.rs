use game_common::module::ModuleId;
use game_ui::reactive::Scope;
use game_ui::render::style::{Background, Bounds, Size, SizeVec2, Style};
use game_ui::{component, view};

use game_ui::widgets::*;

#[component]
pub fn CreateModule(cx: &Scope) -> Scope {
    let root = view! {
        cx,
        <Container style={Style::default()}>
        </Container>
    };

    let id = ModuleId::random();

    view! {
        root,
        <Text text={"ID".into()}>
        </Text>
    };

    view! {
        root,
        <Text text={id.to_string().into()}>
        </Text>
    };

    view! {
        root,
        <Text text={"Name".into()}>
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
        root,
        <Input value={String::new()} style={style}>
        </Input>
    };

    view! {
        root,
        <Button style={Style::default()} on_click={on_create().into()}>
            <Text text={"Create".into()}>
            </Text>
        </Button>
    };

    view! {
        root,
        <Button style={Style::default()} on_click={on_cancel().into()}>
            <Text text={"Cancel".into()}>
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
