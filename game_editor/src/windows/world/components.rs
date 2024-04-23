use std::sync::mpsc;

use game_common::components::components::{Components, RawComponent};
use game_common::components::{Decode, DirectionalLight, MeshInstance, Transform};
use game_ui::reactive::{ReadSignal, Scope};
use game_ui::style::{Background, Bounds, Growth, Padding, Size, SizeVec2, Style};
use game_ui::widgets::{Button, Container, Input, Text, Widget};
use game_wasm::components::Component;
use game_wasm::encoding::BinaryWriter;
use image::Rgba;

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

                        let style = Style {
                            padding: Padding::splat(Size::Pixels(2)),
                            background: Background::RED,
                            ..Default::default()
                        };

                        for (index, val) in transform.translation.to_array().into_iter().enumerate()
                        {
                            let writer = self.writer.clone();

                            root.append(
                                Input::new()
                                    .value(val.to_string())
                                    .style(style.clone())
                                    .on_change(move |value: String| {
                                        let mut transform = transform;

                                        if let Ok(value) = value.parse::<f32>() {
                                            match index {
                                                0 => transform.translation.x = value,
                                                1 => transform.translation.y = value,
                                                2 => transform.translation.z = value,
                                                _ => unreachable!(),
                                            }

                                            let (fields, data) =
                                                BinaryWriter::new().encoded(&transform);
                                            let component = RawComponent::new(data, fields);

                                            writer
                                                .send(Event::UpdateComponent(
                                                    Transform::ID,
                                                    component,
                                                ))
                                                .unwrap();
                                        }
                                    }),
                            );
                        }

                        root.append(Input::new().value(transform.translation.x.to_string()));
                        root.append(Input::new().value(transform.translation.y.to_string()));
                        root.append(Input::new().value(transform.translation.z.to_string()));

                        root.append(Input::new().value(transform.rotation.x.to_string()));
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
