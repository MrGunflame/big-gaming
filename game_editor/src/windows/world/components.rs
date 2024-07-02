use std::collections::VecDeque;
use std::fmt::Display;
use std::str::FromStr;
use std::sync::{mpsc, Arc};

use game_common::components::components::RawComponent;
use game_common::reflection::{ComponentDescriptor, FieldKind};
use game_core::modules::Modules;
use game_data::record::RecordKind;
use game_ui::reactive::Context;
use game_ui::style::{Background, Bounds, Color, Direction, Growth, Size, SizeVec2, Style};
use game_ui::widgets::{Button, Callback, Container, Input, Text, Widget};
use game_wasm::world::RecordReference;
use parking_lot::Mutex;

use super::{Event, SceneState};

#[derive(Clone, Debug)]
pub struct ComponentsPanel {
    pub state: Arc<Mutex<SceneState>>,
    pub writer: mpsc::Sender<Event>,
    pub modules: Modules,
}

impl Widget for ComponentsPanel {
    fn mount<T>(self, parent: &Context<T>) -> Context<()> {
        let style = Style {
            background: Background::GRAY,
            growth: Growth::splat(1.0),
            bounds: Bounds::exact(SizeVec2 {
                x: Size::Pixels(300),
                y: Size::Pixels(2000),
            }),
            ..Default::default()
        };

        let root = Container::new().style(style.clone()).mount(parent);

        let root_ctx = Arc::new(Mutex::new(root.clone()));
        {
            let root_ctx = root_ctx.clone();
            let state = self.state.clone();
            let modules = self.modules.clone();
            let writer = self.writer.clone();
            self.state.lock().components_changed = Callback::from(move |()| {
                mount_component_panel(&root_ctx, &state, &modules, &writer);
            });
        }

        mount_component_panel(&root_ctx, &self.state, &self.modules, &self.writer);

        root
    }
}

fn mount_component_panel(
    parent: &Arc<Mutex<Context<()>>>,
    state: &Arc<Mutex<SceneState>>,
    modules: &Modules,
    writer: &mpsc::Sender<Event>,
) {
    let parent_ctx = parent.lock();
    let state = state.lock();

    parent_ctx.clear_children();

    let root = Container::new().mount(&parent_ctx);

    for (id, component) in state.components.iter() {
        let component_container = Container::new().mount(&root);

        let Some((descriptor, name)) = get_component_descriptor_and_name(modules, id) else {
            continue;
        };

        render_component(
            &component_container,
            id,
            name,
            descriptor,
            writer.clone(),
            component,
        );
    }

    let button = Button::new().mount(&root);
    Text::new("Add Component").mount(&button);
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

fn display_value<T, F>(ctx: &Context<()>, color: Color, label: &str, value: T, on_change: F)
where
    T: Display + FromStr + 'static,
    F: Into<Callback<T>>,
{
    let on_change = on_change.into();

    let root = Container::new()
        .style(Style {
            direction: Direction::Column,
            ..Default::default()
        })
        .mount(ctx);

    let color_box = Container::new()
        .style(Style {
            background: Background::Color(color.0),
            growth: Growth::y(1.0),
            ..Default::default()
        })
        .mount(&root);
    Text::new(label).mount(&color_box);

    Input::new()
        .value(value.to_string())
        .on_change(move |value: String| {
            if let Ok(value) = value.parse::<T>() {
                on_change.call(value);
            }
        })
        .mount(&root);
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
    ctx: &Context<()>,
    id: RecordReference,
    name: &str,
    descriptor: ComponentDescriptor,
    writer: mpsc::Sender<Event>,
    component: &RawComponent,
) {
    Text::new(name).mount(ctx);

    let mut offset = 0;

    let mut queue = VecDeque::new();

    for index in descriptor.root() {
        let field = descriptor.get(*index).unwrap();
        queue.push_back((ctx.clone(), field));
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

                // FIXME: Hardcoded colors for translation/rotation fields
                // for now.
                let color = match field.name.as_str() {
                    "x" | "X" => COLOR_X,
                    "y" | "Y" => COLOR_Y,
                    "z" | "Z" => COLOR_Z,
                    "w" | "W" => COLOR_W,
                    _ => COLOR_X,
                };

                let component = component.clone();
                let writer = writer.clone();
                display_value(ctx, color, &field.name, value, move |mut value: i64| {
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
                display_value(ctx, COLOR_X, &field.name, value, move |value: f64| {
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
                let root = Text::new(field.name.clone()).mount(&parent);

                for index in val.iter().rev() {
                    let field = descriptor.get(*index).unwrap();
                    queue.push_front((root.clone(), field));
                }
            }
        }
    }
}
