use game_data::record::RecordKind;
use game_input::mouse::MouseButtonInput;
use game_ui::events::Context;
use game_ui::reactive::{create_signal, Scope};
use game_ui::render::style::{
    Background, Bounds, Direction, Justify, Padding, Size, SizeVec2, Style,
};
use game_ui::{component, view};

use game_ui::widgets::*;

#[component]
pub fn CreateRecord(cx: &Scope, kind: RecordKind) -> Scope {
    let (name, set_name) = create_signal(cx, String::new());

    let root = view! {
        cx,
        <Container style={Style{ padding: Padding::splat(Size::Pixels(5.0)), justify: Justify::SpaceBetween, ..Default::default() }}>
        </Container>
    };

    let metadata = view! {
        root,
        <Container style={Style { direction: Direction::Column, ..Default::default() }}>
        </Container>
    };

    let name_col = view! {
        metadata,
        <Container style={Style::default()}>
        </Container>
    };

    for text in ["Module", "ID", "Name"] {
        view! {
            name_col,
            <Text text={text.into()}>
            </Text>
        };
    }

    let val_col = view! {
        metadata,
        <Container style={Style::default()}>
        </Container>
    };

    view! {
        val_col,
        <Text text={"TODO".into()}>
        </Text>
    };

    view! {
        val_col,
        <Text text={"TODO".into()}>
        </Text>
    };

    let style = Style {
        bounds: Bounds {
            min: SizeVec2 {
                x: Size::Pixels(100.0),
                y: Size::Pixels(20.0),
            },
            ..Default::default()
        },
        background: Background::GRAY,
        ..Default::default()
    };

    view! {
        val_col,
        <Input value={name.get_untracked()} on_change={set_name.into()} style={style}>
        </Input>
    };

    view! {
        root,
        <Button style={Style::default()} on_click={create_record().into()}>
            <Text text={"OK".into()}>
            </Text>
        </Button>
    };

    root
}

fn create_record() -> Box<dyn Fn(Context<MouseButtonInput>) + Send + Sync + 'static> {
    Box::new(move |ctx| {
        ctx.window.close();
    })
}
