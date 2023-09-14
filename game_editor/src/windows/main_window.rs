use game_ui::reactive::Scope;
use game_ui::style::{Background, BorderRadius, Bounds, Size, SizeVec2, Style};
use game_ui::widgets::{Callback, Container, Widget};

use crate::state::EditorState;
use crate::widgets::header::{ActionButton, Header};
use crate::windows::SpawnWindow;

pub struct MainWindow {
    pub state: EditorState,
}

impl Widget for MainWindow {
    fn build(self, cx: &Scope) -> Scope {
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

        cx.append(Header { buttons });

        let style = Style {
            background: Background::AQUA,
            bounds: Bounds {
                min: SizeVec2::splat(Size::Pixels(64)),
                max: SizeVec2::splat(Size::Pixels(64)),
            },
            border_radius: BorderRadius::splat(Size::Pixels(16)),
            ..Default::default()
        };

        cx.append(Container::new().style(style));

        cx.clone()
    }
}
