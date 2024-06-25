use game_ui::reactive::Context;
use game_ui::style::{Background, BorderRadius, Bounds, Size, SizeVec2, Style};
use game_ui::widgets::{Callback, Container, Widget};

use crate::state::EditorState;
use crate::widgets::header::{ActionButton, Header};
// use crate::widgets::header::{ActionButton, Header};
use crate::windows::SpawnWindow;

pub struct MainWindow {
    pub state: EditorState,
}

impl Widget for MainWindow {
    fn mount<T>(self, parent: &Context<T>) -> Context<()> {
        let buttons = vec![
            ActionButton {
                label: "Modules".to_owned(),
                on_click: {
                    let state = self.state.clone();

                    Callback::from(move |_| {
                        let _ = state.spawn_windows.send(SpawnWindow::Modules);
                    })
                },
            },
            ActionButton {
                label: "Records".to_owned(),
                on_click: {
                    let state = self.state.clone();

                    Callback::from(move |_| {
                        let _ = state.spawn_windows.send(SpawnWindow::Records);
                    })
                },
            },
            ActionButton {
                label: "View".to_owned(),
                on_click: {
                    let state = self.state.clone();

                    Callback::from(move |_| {
                        let _ = state.spawn_windows.send(SpawnWindow::View);
                    })
                },
            },
        ];

        let root = Container::new().mount(parent);
        Header { buttons }.mount(&root);

        root
    }
}
