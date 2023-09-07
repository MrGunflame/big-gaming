use parking_lot::Mutex;
use winit::event::VirtualKeyCode;

use crate::events::{ElementEventHandlers, EventHandlers};
use crate::reactive::{create_effect, create_signal, Node, NodeId, Scope, WriteSignal};
use crate::render::{Element, ElementBody, Text};
use crate::style::{Background, Bounds, Size, SizeVec2, Style};

use super::{Callback, Input, Widget};

pub struct SelectionProps {
    pub options: Vec<String>,
    /// default
    pub value: Option<usize>,
    pub on_change: SelectionChangeHandler,
}

pub struct Selection {
    options: Vec<String>,
    value: Option<usize>,
    on_change: Option<Callback<usize>>,
}

impl Selection {
    pub fn new() -> Self {
        Self {
            options: Vec::new(),
            value: None,
            on_change: None,
        }
    }

    pub fn options(mut self, options: Vec<String>) -> Self {
        self.options = options;
        self
    }

    pub fn value(mut self, value: Option<usize>) -> Self {
        self.value = value;
        self
    }

    pub fn on_change<F>(mut self, on_change: F) -> Self
    where
        F: Into<Callback<usize>>,
    {
        self.on_change = Some(on_change.into());
        self
    }
}

impl Widget for Selection {
    fn build(self, cx: &Scope) -> Scope {
        let num_options = self.options.len();

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

        let input = root.append(
            Input::new()
                .value(match self.value {
                    Some(index) => self.options[index].clone(),
                    None => String::new(),
                })
                .on_change({
                    let set_value = set_value.clone();
                    move |val| set_value.set(Value::Search(val))
                }),
        );

        {
            let value = value.clone();
            create_effect(&root, move || {
                let value = value.get();

                match value {
                    Value::Option(_) => {
                        let style = Style {
                            bounds: Bounds {
                                min: SizeVec2 {
                                    x: Size::Pixels(50),
                                    y: Size::Pixels(20),
                                },
                                max: SizeVec2 {
                                    x: Size::INFINITY,
                                    y: Size::INFINITY,
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
                                    x: Size::Pixels(50),
                                    y: Size::Pixels(20),
                                },
                                max: SizeVec2 {
                                    x: Size::INFINITY,
                                    y: Size::INFINITY,
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
        create_effect(&root, move || {
            let state = state.get();

            let mut ids = ids.lock();
            for id in &*ids {
                cx.remove(*id);
            }
            ids.clear();

            if state {
                for (index, opt) in self.options.iter().enumerate() {
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

        if let Some(cb) = self.on_change {
            create_effect(&root, move || {
                let value = value.get();

                if let Value::Option(index) = value {
                    // The index must be valid for `self.options`.
                    debug_assert!(index <= num_options);

                    (cb)(index);
                }
            });
        }

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
