use parking_lot::Mutex;
use winit::event::VirtualKeyCode;

use crate::events::{ElementEventHandlers, EventHandlers};
use crate::reactive::{create_effect, create_signal, Node, NodeId, Scope, WriteSignal};
use crate::render::style::{Background, Bounds, Size, SizeVec2, Style};
use crate::render::{Element, ElementBody, Text};

use super::{Component, Input, InputProps};

pub struct SelectionProps {
    pub options: Vec<String>,
    /// default
    pub value: Option<usize>,
    pub on_change: SelectionChangeHandler,
}

pub struct Selection;

impl Component for Selection {
    type Properties = SelectionProps;

    fn render(cx: &Scope, props: Self::Properties) -> Scope {
        let (state, set_state) = create_signal(cx, false);
        let (value, set_value) = create_signal(cx, Value::default());

        let root = cx.push(Node {
            element: Element {
                body: ElementBody::Container(),
                style: Style::default(),
            },
            events: ElementEventHandlers {
                global: EventHandlers {
                    keyboard_input: {
                        let set_state = set_state.clone();
                        Some(Box::new(move |ctx| {
                            if ctx.event.key_code == Some(VirtualKeyCode::Escape) {
                                set_state.update(|state| *state = false);
                            }
                        }))
                    },
                    ..Default::default()
                },
                local: EventHandlers {
                    mouse_button_input: {
                        let set_state = set_state.clone();
                        Some(Box::new(move |ctx| {
                            set_state.update(|state| *state = true);
                        }))
                    },
                    ..Default::default()
                },
            },
        });

        let input = Input::render(
            &root,
            InputProps {
                value: match props.value {
                    Some(index) => props.options[index].clone(),
                    None => String::new(),
                },
                on_change: {
                    let set_value = set_value.clone();

                    Box::new(move |val| {
                        set_value.update(|v| *v = Value::Search(val));
                    })
                }
                .into(),
                style: Style::default(),
            },
        );

        {
            let value = value.clone();
            create_effect(&root, move |_| {
                let value = value.get();

                match value {
                    Value::Option(_) => {
                        let style = Style {
                            bounds: Bounds {
                                min: SizeVec2 {
                                    x: Size::Pixels(50.0),
                                    y: Size::Pixels(20.0),
                                },
                                max: SizeVec2 {
                                    x: Size::Pixels(f32::INFINITY),
                                    y: Size::Pixels(f32::INFINITY),
                                },
                            },
                            background: Background::GRAY,
                            ..Default::default()
                        };

                        input.set_style(input.id().unwrap(), style);
                    }
                    Value::Search(_) => {
                        let style = Style {
                            bounds: Bounds {
                                min: SizeVec2 {
                                    x: Size::Pixels(50.0),
                                    y: Size::Pixels(20.0),
                                },
                                max: SizeVec2 {
                                    x: Size::Pixels(f32::INFINITY),
                                    y: Size::Pixels(f32::INFINITY),
                                },
                            },
                            background: Background::RED,
                            ..Default::default()
                        };

                        input.set_style(input.id().unwrap(), style);
                    }
                }
            });
        }

        let ids: Mutex<Vec<NodeId>> = Mutex::new(vec![]);
        let cx = root.clone();
        create_effect(&root, move |_| {
            let state = state.get();

            let mut ids = ids.lock();
            for id in &*ids {
                cx.remove(*id);
            }
            ids.clear();

            if state {
                for (index, opt) in props.options.iter().enumerate() {
                    let node = cx.push(Node {
                        element: Element {
                            body: ElementBody::Text(Text {
                                text: opt.clone(),
                                size: 15.0,
                            }),
                            style: Style::default(),
                        },
                        events: ElementEventHandlers {
                            local: EventHandlers {
                                mouse_button_input: {
                                    let set_state = set_state.clone();
                                    let set_value = set_value.clone();

                                    Some(Box::new(move |ctx| {
                                        set_state.update(|state| *state = false);
                                        set_value.update(|val| *val = Value::Option(index));
                                    }))
                                },
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                    });

                    ids.push(node.id().unwrap());
                }
            }
        });

        create_effect(&root, move |_| {
            let value = value.get();

            if let Value::Option(index) = value {
                (props.on_change.0)(index);
            }
        });

        root
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Value {
    /// Option `n` is selected (valid).
    Option(usize),
    /// Search string (invalid).
    Search(String),
}

impl Default for Value {
    fn default() -> Self {
        Self::Search(String::new())
    }
}

pub struct SelectionChangeHandler(Box<dyn Fn(usize) + Send + Sync + 'static>);

impl<F> From<F> for SelectionChangeHandler
where
    F: Fn(usize) + Send + Sync + 'static,
{
    fn from(value: F) -> Self {
        Self(Box::new(value))
    }
}

impl From<WriteSignal<usize>> for SelectionChangeHandler {
    fn from(writer: WriteSignal<usize>) -> Self {
        Self(Box::new(move |index| {
            writer.update(|val| *val = index);
        }))
    }
}
