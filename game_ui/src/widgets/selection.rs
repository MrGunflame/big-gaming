use std::sync::Arc;

use game_input::mouse::MouseButtonInput;
use glam::UVec2;
use parking_lot::Mutex;

use crate::reactive::Context;
use crate::style::{Position, Style};

use super::{Button, Callback, Container, Input, Text, Widget};

pub struct Selection {
    pub options: Vec<String>,
    pub on_change: Callback<usize>,
}

impl Widget for Selection {
    fn mount<T>(self, parent: &Context<T>) -> Context<()> {
        let wrapper = Container::new().mount(parent);

        let options_wrapper = Container::new().mount(&wrapper);
        let options_wrapper = Arc::new(Mutex::new(options_wrapper));

        let filter = Arc::new(Mutex::new(String::new()));

        let input_wrapper = Container::new().mount(&wrapper);
        let input_wrapper = Arc::new(Mutex::new(input_wrapper));

        let input = Arc::new(Mutex::new(None));
        let wrapper_mux = Arc::new(Mutex::new(wrapper.clone()));

        let options = Arc::new(self.options);
        let on_change = self.on_change.clone();

        {
            let input_ctx = Input::new()
                .on_change({
                    let options = options.clone();
                    let on_change = on_change.clone();
                    let filter = filter.clone();
                    let options_wrapper = options_wrapper.clone();
                    let wrapper_mux = wrapper_mux.clone();
                    let input = input.clone();
                    let input_wrapper = input_wrapper.clone();

                    move |value| {
                        *filter.lock() = value;

                        mount_selector(
                            &mut options_wrapper.lock(),
                            &input_wrapper.lock(),
                            &wrapper_mux.lock(),
                            &input,
                            &filter,
                            &options,
                            &on_change,
                        );
                    }
                })
                .mount(&input_wrapper.lock());

            *input.lock() = Some(input_ctx);
        }

        {
            let wrapper_mux = wrapper_mux.clone();
            parent
                .document()
                .register(move |_ctx: Context<MouseButtonInput>| {
                    mount_selector(
                        &mut options_wrapper.lock(),
                        &input_wrapper.lock(),
                        &wrapper_mux.lock(),
                        &input,
                        &filter,
                        &options,
                        &on_change,
                    )
                });
        }

        wrapper
    }
}

fn mount_selector(
    options_wrapper: &mut Context<()>,
    input_wrapper: &Context<()>,
    wrapper: &Context<()>,
    input: &Arc<Mutex<Option<Context<()>>>>,
    filter: &Arc<Mutex<String>>,
    options: &[String],
    on_change: &Callback<usize>,
) {
    let input_id = {
        let Some(input_ctx) = &*input.lock() else {
            return;
        };
        input_ctx.node().unwrap()
    };

    let layout = wrapper.layout(input_id).unwrap();

    options_wrapper.remove(options_wrapper.node.unwrap());
    if !layout.contains(wrapper.cursor().as_uvec2()) {
        return;
    }

    let style = Style {
        position: Position::Absolute(UVec2::new(layout.min.x, layout.max.y)),
        ..Default::default()
    };
    *options_wrapper = Container::new().style(style).mount(&wrapper);
    let filter_string = filter.lock().to_lowercase();
    for (index, option) in options.iter().enumerate() {
        if !option.to_lowercase().contains(&filter_string) {
            continue;
        }

        let input_wrapper = input_wrapper.clone();
        let filter = filter.clone();
        let input = input.clone();
        let on_change = on_change.clone();
        let option2 = option.to_owned();
        let button = Button::new()
            .on_click(move |()| {
                input_wrapper.clear_children();
                let filter = filter.clone();
                on_change.call(index);
                let option2 = option2.clone();

                let c = Input::new()
                    .value(option2)
                    .on_change(move |value| {
                        *filter.lock() = value;
                    })
                    .mount(&input_wrapper);
                *input.lock() = Some(c);
            })
            .mount(&options_wrapper);

        Text::new(option).mount(&button);
    }
}
