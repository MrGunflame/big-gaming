use std::collections::HashSet;
use std::sync::mpsc;

use game_core::hierarchy::{Hierarchy, Key};
use game_ui::reactive::{ReadSignal, Scope};
use game_ui::style::{Background, Style};
use game_ui::widgets::{Button, Container, Text, Widget};
use parking_lot::Mutex;

use crate::state::EditorState;
use crate::windows::SpawnWindow;

use super::node::Node;
use super::Event;

#[derive(Debug)]
pub struct NodeHierarchy {
    pub writer: mpsc::Sender<Event>,
    /// All nodes in the tree.
    pub nodes: ReadSignal<Hierarchy<Node>>,
    /// All currently selected nodes.
    pub selection: ReadSignal<HashSet<Key>>,
    pub state: EditorState,
}

impl Widget for NodeHierarchy {
    fn build(self, cx: &Scope) -> Scope {
        let root = cx.append(Container::new());

        {
            let header = root.append(Container::new());

            let writer = self.writer.clone();
            let on_click = move |_ctx| {
                self.state
                    .spawn_windows
                    .send(SpawnWindow::SpawnEntity(writer.clone()))
                    .unwrap();
            };

            let new = header.append(Button::new().on_click(on_click));
            new.append(Text::new().text("New".to_owned()));
        }

        {
            let nodes = Mutex::new(vec![]);
            let cx2 = root.clone();

            root.create_effect(move || {
                let selection = self.selection.get();

                let mut nodes = nodes.lock();
                for id in nodes.drain(..) {
                    cx2.remove(id);
                }

                for (key, node) in self.nodes.get().iter() {
                    let is_selected = selection.contains(&key);

                    let writer = self.writer.clone();
                    let on_click = move |_ctx| {
                        writer
                            .send(Event::UpdateSelection {
                                node: key,
                                additive: false,
                            })
                            .unwrap();
                    };

                    let style = Style {
                        background: if is_selected {
                            Background::YELLOW
                        } else {
                            Background::None
                        },
                        ..Default::default()
                    };

                    let button = cx2.append(Button::new().style(style).on_click(on_click));
                    button.append(Text::new().text(node.name.to_owned()));

                    nodes.push(button.id().unwrap());
                }
            });
        }

        root
    }
}
