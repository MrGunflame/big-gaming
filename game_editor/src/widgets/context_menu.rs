//! Context-menu (right-click)

use game_ui::events::{ElementEventHandlers, EventHandlers};
use game_ui::reactive::{create_effect, create_signal, Node, Scope};

use game_ui::component;
use game_ui::render::style::{Growth, Position, Style};
use game_ui::render::{Element, ElementBody};
use game_window::events::VirtualKeyCode;
use glam::Vec2;

#[component]
pub fn ContextMenu(cx: &Scope, spawn_menu: Box<dyn Fn(Scope) + Send + Sync + 'static>) -> Scope {
    let (state, set_state) = create_signal(
        cx,
        State {
            is_active: false,
            menu_cx: None,
            position: Vec2::splat(0.0),
        },
    );

    let wrapper = cx.push(Node {
        element: Element {
            body: ElementBody::Container(),
            style: Style {
                growth: Growth::splat(1.0),
                ..Default::default()
            },
        },
        events: ElementEventHandlers {
            local: EventHandlers {
                mouse_button_input: Some(Box::new({
                    let set_state = set_state.clone();

                    move |ctx| {
                        if ctx.event.state.is_pressed() && ctx.event.button.is_right() {
                            set_state.update(|state| {
                                state.is_active = true;
                                state.position = ctx.cursor.position();
                            });
                        }
                    }
                })),
                ..Default::default()
            },
            ..Default::default()
        },
    });

    let cx2 = cx.clone();
    create_effect(cx, move |_| {
        let set_state = set_state.clone();

        state.with_mut(|state| match &state.menu_cx {
            Some(cx) => {
                if !state.is_active {
                    cx2.remove(cx.id().unwrap());
                    state.menu_cx = None;
                }
            }
            None => {
                if state.is_active {
                    let menu = cx2.push(Node {
                        element: Element {
                            body: ElementBody::Container(),
                            style: Style {
                                position: Position::Absolute(state.position),
                                ..Default::default()
                            },
                        },
                        events: ElementEventHandlers {
                            local: EventHandlers {
                                mouse_button_input: Some(Box::new({
                                    let set_state = set_state.clone();

                                    move |event| {
                                        set_state.update(|state| state.is_active = true);
                                    }
                                })),
                                ..Default::default()
                            },
                            global: EventHandlers {
                                mouse_button_input: Some(Box::new(move |ctx| {
                                    //set_state.update(|state| state.is_active = false);
                                })),
                                keyboard_input: Some(Box::new(move |ctx| {
                                    if ctx.event.state.is_pressed()
                                        && ctx.event.key_code == Some(VirtualKeyCode::Escape)
                                    {
                                        set_state.update(|state| state.is_active = false);
                                    }
                                })),
                                ..Default::default()
                            },
                        },
                    });

                    state.menu_cx = Some(menu.clone());

                    spawn_menu(menu);
                }
            }
        });
    });

    wrapper
}

#[derive(Clone, Debug)]
struct State {
    is_active: bool,
    position: Vec2,
    // Menu scope if displayed.
    menu_cx: Option<Scope>,
}
