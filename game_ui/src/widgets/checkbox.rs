use image::{ImageBuffer, Rgba};
use parking_lot::Mutex;

use crate::events::{ElementEventHandlers, EventHandlers};
use crate::reactive::{create_effect, create_signal, Node, Scope};
use crate::render::{Element, ElementBody, Image};
use crate::style::Style;

use super::{Callback, Widget};

pub struct Checkbox {
    is_checked: bool,
    style: Style,
    on_change: Callback<bool>,
}

impl Widget for Checkbox {
    fn build(self, cx: &Scope) -> Scope {
        let (state, set_state) = create_signal(cx, self.is_checked);

        let root = cx.push(Node {
            element: Element {
                body: ElementBody::Container(),
                style: self.style,
            },
            events: ElementEventHandlers {
                local: EventHandlers {
                    mouse_button_input: Some(Box::new(move |ctx| {
                        if ctx.event.state.is_pressed() && ctx.event.button.is_left() {
                            set_state.update(|val| *val ^= true);
                        }
                    })),
                    ..Default::default()
                },
                ..Default::default()
            },
        });

        let id = Mutex::new(None);
        let cx = root.clone();
        create_effect(&root, move || {
            let state = state.get();

            let mut id = id.lock();

            if let Some(id) = &*id {
                cx.remove(*id);
            }

            let image = if state {
                checkbox_checked()
            } else {
                checkbox_unchecked()
            };

            let checkbox = cx.push(Node {
                element: Element {
                    body: ElementBody::Image(Image { image }),
                    style: Style::default(),
                },
                events: Default::default(),
            });

            // Skip update for the initial value.
            if id.is_some() {
                (self.on_change)(state);
            }

            *id = Some(checkbox.id().unwrap());
        });

        root
    }
}

fn checkbox_unchecked() -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let mut buffer = ImageBuffer::new(32, 32);

    for x in 0..buffer.width() {
        buffer.put_pixel(x, 0, Rgba([255, 255, 255, 255]));
        buffer.put_pixel(x, buffer.height() - 1, Rgba([255, 255, 255, 255]));
    }

    for y in 0..buffer.height() {
        buffer.put_pixel(0, y, Rgba([255, 255, 255, 255]));
        buffer.put_pixel(buffer.width() - 1, y, Rgba([255, 255, 255, 255]));
    }

    buffer
}

fn checkbox_checked() -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let mut buffer = checkbox_unchecked();

    for x in 0..buffer.width() {
        buffer.put_pixel(x, x, Rgba([255, 255, 255, 255]));
    }

    buffer
}
