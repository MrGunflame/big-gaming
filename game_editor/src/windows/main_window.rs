use game_ui::reactive::Scope;
use game_ui::render::style::{Background, BorderRadius, Bounds, Size, SizeVec2, Style};
use game_ui::{component, widgets::*};

use crate::state::EditorState;
use crate::widgets::tool_bar::*;
use crate::windows::SpawnWindow;

#[component]
pub fn MainWindow(cx: &Scope, state: EditorState) -> Scope {
    let buttons = vec![
        ActionButton {
            label: "Modules".to_owned(),
            on_click: {
                let state = state.clone();

                Box::new(move |_| {
                    let _ = state.spawn_windows.send(SpawnWindow::Modules);
                })
            },
        },
        ActionButton {
            label: "Records".to_owned(),
            on_click: {
                let state = state.clone();

                Box::new(move |_| {
                    let _ = state.spawn_windows.send(SpawnWindow::Records);
                })
            },
        },
        ActionButton {
            label: "View".to_owned(),
            on_click: {
                let state = state.clone();

                Box::new(move |_| {
                    let _ = state.spawn_windows.send(SpawnWindow::View);
                })
            },
        },
    ];

    game_ui::view! {
        cx,
        <ToolBar buttons={buttons}>
        </ToolBar>
    };

    let style = Style {
        background: Background::AQUA,
        bounds: Bounds {
            min: SizeVec2::splat(Size::Pixels(64.0)),
            max: SizeVec2::splat(Size::Pixels(64.0)),
        },
        border_radius: BorderRadius::splat(Size::Pixels(16.0)),
        ..Default::default()
    };

    game_ui::view! {cx,
        <Container style={style}>
        </Container>
    };

    cx.clone()
}
