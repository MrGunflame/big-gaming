use std::collections::VecDeque;
use std::fmt::Display;
use std::str::FromStr;
use std::sync::mpsc;

use game_common::components::components::{Components, RawComponent};
use game_common::components::{Decode, DirectionalLight, MeshInstance, PointLight, Transform};
use game_common::reflection::{ComponentDescriptor, FieldKind};
use game_core::modules::Modules;
use game_data::record::RecordKind;
use game_ui::reactive::{ReadSignal, Scope};
use game_ui::style::{Background, Bounds, Color, Direction, Growth, Size, SizeVec2, Style};
use game_ui::widgets::{Button, Callback, Container, Input, Text, Widget};
use game_wasm::components::Component;
use game_wasm::encoding::BinaryWriter;
use game_wasm::world::RecordReference;

use super::Event;

#[derive(Clone, Debug)]
pub struct ComponentsPanel {
    pub components: ReadSignal<Components>,
    pub writer: mpsc::Sender<Event>,
    pub modules: Modules,
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

                let Some((descriptor, name)) = get_component_descriptor_and_name(&self.modules, id)
                else {
                    continue;
                };

                render_component(
                    &component_container,
                    id,
                    name,
                    descriptor,
                    self.writer.clone(),
                    component,
                );
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

fn get_component_descriptor_and_name(
    modules: &Modules,
    id: RecordReference,
) -> Option<(ComponentDescriptor, &str)> {
    let record = modules.get(id.module)?.records.get(id.record)?;
    if record.kind != RecordKind::COMPONENT {
        None
    } else {
        Some((ComponentDescriptor::from_bytes(&record.data), &record.name))
    }
}

fn render_component(
    cx: &Scope,
    id: RecordReference,
    name: &str,
    descriptor: ComponentDescriptor,
    writer: mpsc::Sender<Event>,
    component: &RawComponent,
) {
    cx.append(Text::new().text(name.to_string()));

    let mut offset = 0;

    let mut queue = VecDeque::new();

    for index in descriptor.root() {
        let field = descriptor.get(*index).unwrap();
        queue.push_back((cx.clone(), field));
    }

    while let Some((parent, field)) = queue.pop_front() {
        match &field.kind {
            FieldKind::Int(val) => {
                let field_len = usize::from(val.bits) / 8;
                let bits = val.bits;
                let is_signed = val.is_signed;

                let bytes = &component.as_bytes()[offset..offset + field_len];
                let value = match (bits, is_signed) {
                    (8, false) => u8::from_le_bytes(bytes.try_into().unwrap()) as i64,
                    (8, true) => i8::from_le_bytes(bytes.try_into().unwrap()) as i64,
                    (16, false) => u16::from_le_bytes(bytes.try_into().unwrap()) as i64,
                    (16, true) => u16::from_le_bytes(bytes.try_into().unwrap()) as i64,
                    (32, false) => u32::from_le_bytes(bytes.try_into().unwrap()) as i64,
                    (32, true) => i32::from_le_bytes(bytes.try_into().unwrap()) as i64,
                    (64, false) => u64::from_le_bytes(bytes.try_into().unwrap()) as i64,
                    (64, true) => i64::from_le_bytes(bytes.try_into().unwrap()),
                    _ => todo!(),
                };

                let component = component.clone();
                let writer = writer.clone();
                display_value(cx, COLOR_X, &field.name, value, move |mut value: i64| {
                    let mut bytes = component.as_bytes().to_vec();
                    let fields = component.fields().to_vec();

                    if !is_signed {
                        value = value.abs();
                    }

                    match bits {
                        8 => {
                            bytes[offset..offset + field_len]
                                .copy_from_slice(&(value as u8).to_le_bytes());
                        }
                        16 => {
                            bytes[offset..offset + field_len]
                                .copy_from_slice(&(value as u16).to_le_bytes());
                        }
                        32 => {
                            bytes[offset..offset + field_len]
                                .copy_from_slice(&(value as u32).to_le_bytes());
                        }
                        64 => {
                            bytes[offset..offset + field_len]
                                .copy_from_slice(&(value as u64).to_le_bytes());
                        }
                        _ => todo!(),
                    }

                    writer
                        .send(Event::UpdateComponent(id, RawComponent::new(bytes, fields)))
                        .unwrap();
                });

                offset += field_len;
            }
            FieldKind::Float(val) => {
                let field_len = usize::from(val.bits) / 8;
                let bits = val.bits;

                let bytes = &component.as_bytes()[offset..offset + field_len];
                let value = match bits {
                    32 => f32::from_le_bytes(bytes.try_into().unwrap()) as f64,
                    64 => f64::from_le_bytes(bytes.try_into().unwrap()),
                    _ => todo!(),
                };

                let component = component.clone();
                let writer = writer.clone();
                display_value(cx, COLOR_X, &field.name, value, move |value: f64| {
                    let mut bytes = component.as_bytes().to_vec();
                    let fields = component.fields().to_vec();

                    match bits {
                        32 => {
                            bytes[offset..offset + field_len]
                                .copy_from_slice(&(value as f32).to_le_bytes());
                        }
                        64 => {
                            bytes[offset..offset + field_len].copy_from_slice(&value.to_le_bytes());
                        }
                        _ => todo!(),
                    }

                    writer
                        .send(Event::UpdateComponent(id, RawComponent::new(bytes, fields)))
                        .unwrap();
                });

                offset += field_len;
            }
            FieldKind::Struct(val) => {
                let root = parent.append(Text::new().text(field.name.to_string()));

                for index in val.iter().rev() {
                    let field = descriptor.get(*index).unwrap();
                    queue.push_front((root.clone(), field));
                }
            }
        }
    }
}
