use game_ui::events::{ElementEventHandlers, EventHandlers};
use game_ui::reactive::{Node, Scope};
use game_ui::render::{Element, ElementBody};
use game_ui::style::{Bounds, Growth, Padding, Size, SizeVec2, Style};
use game_ui::widgets::{Container, Widget};
use glam::{UVec2, Vec2};

/// A user resizeable area.
pub struct Area {
    size: UVec2,
}

impl Area {
    pub fn new() -> Self {
        Self {
            size: UVec2::splat(0),
        }
    }
}

impl Widget for Area {
    fn build(self, cx: &Scope) -> Scope {
        // We use two nested containers, with the inner one having
        // additional padding. This padded area is the area that
        // tracks mouse cursor movements to resize the container.

        let size = 500;

        let root_style = Style {
            // growth: Growth::splat(1.0),
            bounds: Bounds::exact(SizeVec2::splat(Size::Pixels(size))),
            ..Default::default()
        };

        let inner_style = Style {
            growth: Growth::splat(1.0),
            // padding: Padding::splat(Size::Pixels(5)),
            ..Default::default()
        };

        let (state, set_state) = cx.create_signal(State {
            inside: false,
            inside_inner: false,
            drag: DragState::None,
        });

        // We need to know the ID of the node before we can create
        // the callbacks that modify them.
        // TODO: This could be made more efficient by adding a `push_cyclic`
        // or similar method to `Scope` that allocates an ID before assigning
        // anything to it.
        let root = cx.push(Node {
            element: Element {
                style: Style::default(),
                body: ElementBody::Container,
            },
            events: ElementEventHandlers::default(),
        });

        let cx2 = root.clone();
        let root_id = root.id().unwrap();
        root.update(
            root_id,
            Node {
                element: Element {
                    style: root_style,
                    body: ElementBody::Container,
                },
                events: ElementEventHandlers {
                    local: EventHandlers {
                        cursor_moved: Some(Box::new(move |ctx| {
                            // Rebind in closure.
                            let mut size = size;

                            match state.get_untracked().drag {
                                DragState::None => {}
                                DragState::Start(start) => {
                                    let delta = ctx.event.position.x - start.x;

                                    size = size.checked_add_signed(delta as i32).unwrap();

                                    cx2.set_style(
                                        root_id,
                                        Style {
                                            bounds: Bounds::exact(SizeVec2::splat(Size::Pixels(
                                                size,
                                            ))),
                                            ..Default::default()
                                        },
                                    );
                                }
                            }
                        })),
                        cursor_left: Some({
                            let set_state = set_state.clone();

                            Box::new(move |_ctx| {
                                set_state.update_untracked(|state| state.drag = DragState::None);
                            })
                        }),
                        mouse_button_input: Some(Box::new(move |ctx| {
                            if ctx.event.button.is_left() {
                                let is_dragging = ctx.event.state.is_pressed();
                                set_state.update_untracked(|state| {
                                    match (state.drag, is_dragging) {
                                        (DragState::None, true) => {
                                            state.drag = DragState::Start(ctx.cursor.position());
                                        }
                                        (DragState::Start(_), false) => {
                                            state.drag = DragState::None;
                                        }
                                        _ => (),
                                    }
                                });
                            }
                        })),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            },
        );

        let inner = root.push(Node {
            element: Element {
                style: inner_style,
                body: ElementBody::Container,
            },
            events: ElementEventHandlers {
                local: EventHandlers {
                    cursor_entered: Some(Box::new(|ctx| {})),
                    ..Default::default()
                },
                ..Default::default()
            },
        });

        inner
    }
}

#[derive(Copy, Clone, Debug)]
struct State {
    inside: bool,
    inside_inner: bool,
    drag: DragState,
}

#[derive(Copy, Clone, Debug, Default)]
enum DragState {
    /// Not dragging
    #[default]
    None,
    /// Started dragging
    Start(Vec2),
}
