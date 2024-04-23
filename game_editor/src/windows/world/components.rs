use std::fmt::Display;
use std::str::FromStr;
use std::sync::mpsc;

use game_common::components::components::{Components, RawComponent};
use game_common::components::{Decode, DirectionalLight, MeshInstance, Transform};
use game_ui::reactive::{ReadSignal, Scope};
use game_ui::style::{Background, Bounds, Color, Direction, Growth, Size, SizeVec2, Style};
use game_ui::widgets::{Button, Callback, Container, Input, Text, Widget};
use game_wasm::components::Component;
use game_wasm::encoding::BinaryWriter;

use super::Event;

#[derive(Clone, Debug)]
pub struct ComponentsPanel {
    pub components: ReadSignal<Components>,
    pub writer: mpsc::Sender<Event>,
}

impl Widget for ComponentsPanel {
    fn build(self, cx: &Scope) -> Scope {
        let style = Style {
            background: Background::GRAY,
            growth: Growth::splat(1.0),
            bounds: Bounds::exact(SizeVec2 {
                x: Size::Pixels(300),
                y: Size::Pixels(2000),
            }),
            ..Default::default()
        };

        let root_cx = cx.clone();
        let root = root_cx.append(Container::new().style(style.clone()));
        let mut id = root.id().unwrap();

        cx.create_effect(move || {
            root_cx.remove(id);
            let root = root_cx.append(Container::new().style(style.clone()));
            id = root.id().unwrap();

            let components = self.components.get();

            for (id, component) in components.iter() {
                let component_container = root.append(Container::new());

                match id {
                    Transform::ID => {
                        component_container.append(Text::new().text("Transform".to_string()));

                        let reader = component.reader();
                        let transform = Transform::decode(reader).unwrap();

                        let translation_row =
                            component_container.append(Container::new().style(Style {
                                direction: Direction::Column,
                                ..Default::default()
                            }));

                        for (index, val) in transform.translation.to_array().into_iter().enumerate()
                        {
                            let writer = self.writer.clone();

                            let (color, label) = match index {
                                0 => (COLOR_X, "X"),
                                1 => (COLOR_Y, "Y"),
                                2 => (COLOR_Z, "Z"),
                                _ => unreachable!(),
                            };

                            display_value(&translation_row, color, label, val, move |value| {
                                let mut transform = transform;

                                match index {
                                    0 => transform.translation.x = value,
                                    1 => transform.translation.y = value,
                                    2 => transform.translation.z = value,
                                    _ => unreachable!(),
                                }

                                let (fields, data) = BinaryWriter::new().encoded(&transform);
                                let component = RawComponent::new(data, fields);

                                writer
                                    .send(Event::UpdateComponent(Transform::ID, component))
                                    .unwrap();
                            });
                        }

                        let rotation_row =
                            component_container.append(Container::new().style(Style {
                                direction: Direction::Column,
                                ..Default::default()
                            }));

                        for (index, val) in transform.rotation.to_array().into_iter().enumerate() {
                            let writer = self.writer.clone();

                            let (color, label) = match index {
                                0 => (COLOR_X, "X"),
                                1 => (COLOR_Y, "Y"),
                                2 => (COLOR_Z, "Z"),
                                3 => (COLOR_W, "W"),
                                _ => unreachable!(),
                            };

                            display_value(&rotation_row, color, label, val, move |value| {
                                let mut transform = transform;

                                match index {
                                    0 => transform.rotation.x = value,
                                    1 => transform.rotation.y = value,
                                    2 => transform.rotation.z = value,
                                    3 => transform.rotation.w = value,
                                    _ => unreachable!(),
                                }

                                let (fields, data) = BinaryWriter::new().encoded(&transform);
                                let component = RawComponent::new(data, fields);

                                writer
                                    .send(Event::UpdateComponent(Transform::ID, component))
                                    .unwrap();
                            });
                        }

                        let scale_row = component_container.append(Container::new().style(Style {
                            direction: Direction::Column,
                            ..Default::default()
                        }));

                        for (index, val) in transform.scale.to_array().into_iter().enumerate() {
                            let writer = self.writer.clone();

                            let (color, label) = match index {
                                0 => (COLOR_X, "X"),
                                1 => (COLOR_Y, "Y"),
                                2 => (COLOR_Z, "Z"),
                                _ => unreachable!(),
                            };

                            display_value(&scale_row, color, label, val, move |value| {
                                let mut transform = transform;

                                match index {
                                    0 => transform.scale.x = value,
                                    1 => transform.scale.y = value,
                                    2 => transform.scale.z = value,
                                    _ => unreachable!(),
                                }

                                let (fields, data) = BinaryWriter::new().encoded(&transform);
                                let component = RawComponent::new(data, fields);

                                writer
                                    .send(Event::UpdateComponent(Transform::ID, component))
                                    .unwrap();
                            });
                        }
                    }
                    MeshInstance::ID => {
                        component_container.append(Text::new().text("Mesh Instance".to_string()));

                        let reader = component.reader();
                        let instance = MeshInstance::decode(reader).unwrap();

                        root.append(Input::new().value(instance.path.clone()));
                    }
                    DirectionalLight::ID => {
                        component_container
                            .append(Text::new().text("Directional Light".to_string()));

                        let reader = component.reader();
                        let light = DirectionalLight::decode(reader).unwrap();

                        //COLOR
                        // root.append();
                        root.append(Input::new().value(light.illuminance.to_string()));
                    }
                    _ => (),
                }
            }

            let button = root.append(Button::new());
            button.append(Text::new().text("Add Component".to_string()));
        });

        cx.clone()
    }
}

macro_rules! define_color {
    ($($id:ident = $val:expr),*$(,)?) => {
        $(
            const $id: Color = match Color::from_hex($val) {
                Ok(v) => v,
                Err(_) => panic!("invalid hex"),
            };
        )*
    };
}

define_color! {
    COLOR_X = "d12e19",
    COLOR_Y = "26cc29",
    COLOR_Z = "2692cc",
    COLOR_W = "7b24c1",
}

fn display_value<T, F>(cx: &Scope, color: Color, label: &str, value: T, on_change: F)
where
    T: Display + FromStr + 'static,
    F: Into<Callback<T>>,
{
    let on_change = on_change.into();

    let root = cx.append(Container::new().style(Style {
        direction: Direction::Column,
        ..Default::default()
    }));

    let color_box = root.append(Container::new().style(Style {
        background: Background::Color(color.0),
        growth: Growth::y(1.0),
        ..Default::default()
    }));
    color_box.append(Text::new().text(label.to_string()));

    root.append(
        Input::new()
            .value(value.to_string())
            .on_change(move |value: String| {
                if let Ok(value) = value.parse::<T>() {
                    on_change(value);
                }
            }),
    );
}
