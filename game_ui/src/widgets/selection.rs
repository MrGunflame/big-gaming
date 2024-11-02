use game_input::mouse::MouseButtonInput;
use game_tracing::trace_span;

use crate::runtime::reactive::NodeContext;
use crate::runtime::Context;

use super::{Button, Callback, Container, Input, Text, Widget};

pub struct Selection {
    options: Vec<String>,
    on_change: Callback<usize>,
    value: Option<usize>,
}

impl Selection {
    pub fn new(options: Vec<String>) -> Self {
        Self {
            options,
            on_change: Callback::default(),
            value: None,
        }
    }

    pub fn on_change<T>(mut self, on_change: T) -> Self
    where
        T: Into<Callback<usize>>,
    {
        self.on_change = on_change.into();
        self
    }

    pub fn value(mut self, value: usize) -> Self {
        debug_assert!(value < self.options.len());

        self.value = Some(value);
        self
    }
}

impl Widget for Selection {
    fn mount(self, parent: &Context) -> Context {
        let _span = trace_span!("Selection::mount").entered();

        let default_value = match self.value {
            Some(index) => self.options.get(index).cloned().unwrap_or_default(),
            None => String::new(),
        };

        let root = Container::new().mount(parent);

        let (filter, set_filter) = root.runtime().reactive().create_signal(String::new());
        let (active, set_active) = root.runtime().reactive().create_signal(false);
        let (input_value, set_input_value) = root.runtime().reactive().create_signal(default_value);

        let input_ctx = Container::new().mount(&root);

        {
            let set_filter = set_filter.clone();
            let set_active = set_active.clone();

            let mut input: Option<Context> = None;
            root.runtime()
                .reactive()
                .register_and_schedule_effect(move |ctx: &mut NodeContext| {
                    ctx.subscribe(input_value.id());

                    if let Some(ctx) = input.take() {
                        ctx.remove_self();
                    }

                    let ctx = Input::new()
                        .value(input_value.get())
                        .on_change({
                            let set_filter = set_filter.clone();
                            let set_active = set_active.clone();

                            move |value| {
                                set_filter.set(value);
                                set_active.set(true);
                            }
                        })
                        .mount(&input_ctx);

                    // Open the dropdown menu when the client clicks
                    // inside the input field.
                    let input_ctx = ctx.clone();
                    let set_active = set_active.clone();
                    ctx.document().register_with_parent(
                        ctx.node().unwrap(),
                        move |event: MouseButtonInput| {
                            if !event.button.is_left() || !event.state.is_pressed() {
                                return;
                            }

                            let Some(cursor) = input_ctx.cursor().position() else {
                                return;
                            };

                            let Some(layout) = input_ctx.layout(input_ctx.node().unwrap()) else {
                                return;
                            };

                            if layout.contains(cursor) {
                                set_active.set(true);
                            }
                        },
                    );

                    input = Some(ctx);
                });
        }

        {
            let set_active = set_active.clone();
            let mut buttons: Vec<Context> = Vec::new();
            let root_ctx = root.clone();
            root.runtime()
                .reactive()
                .register_and_schedule_effect(move |ctx: &mut NodeContext| {
                    ctx.subscribe(filter.id());
                    ctx.subscribe(active.id());

                    let filter = filter.get();
                    let active = active.get();

                    for ctx in buttons.drain(..) {
                        ctx.remove_self();
                    }

                    if !active {
                        return;
                    }

                    for (index, option) in self.options.iter().enumerate() {
                        if !option.contains(&filter) {
                            continue;
                        }

                        let button = Button::new()
                            .on_click({
                                let on_change = self.on_change.clone();
                                let set_filter = set_filter.clone();
                                let set_active = set_active.clone();
                                let set_input_value = set_input_value.clone();
                                let option = option.clone();

                                move |()| {
                                    on_change.call(index);
                                    set_filter.set(option.clone());
                                    set_active.set(false);
                                    set_input_value.set(option.clone());
                                }
                            })
                            .mount(&root_ctx);
                        Text::new(option).mount(&button);

                        buttons.push(button);
                    }
                });
        }

        // Close the options dropdown menu when the client clicks
        // outside of the input or options box.
        let ctx = root.clone();
        root.document().register_with_parent(
            root.node().unwrap(),
            move |event: MouseButtonInput| {
                if !event.button.is_left() || !event.state.is_pressed() {
                    return;
                }

                let Some(cursor) = ctx.cursor().position() else {
                    return;
                };

                let Some(layout) = ctx.layout(ctx.node().unwrap()) else {
                    return;
                };

                if !layout.contains(cursor) {
                    set_active.set(false);
                    return;
                }
            },
        );

        root
    }
}
