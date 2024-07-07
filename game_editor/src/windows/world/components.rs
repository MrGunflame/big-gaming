use std::collections::VecDeque;
use std::fmt::{self, Display, Formatter};
use std::str::FromStr;
use std::sync::{mpsc, Arc};

use game_common::components::components::RawComponent;
use game_common::reflection::{ComponentDescriptor, EnumFieldVariant, Field, FieldKind};
use game_core::modules::Modules;
use game_data::record::RecordKind;
use game_ui::reactive::Context;
use game_ui::style::{
    Background, BorderRadius, Bounds, Color, Direction, Growth, Padding, Size, SizeVec2, Style,
};
use game_ui::widgets::{Button, Callback, Container, Input, Selection, Svg, SvgData, Text, Widget};
use game_wasm::world::RecordReference;
use image::Rgba;
use indexmap::IndexMap;
use parking_lot::Mutex;

use super::{Event, SceneState};

const PANEL_COLOR: Color = Color(Rgba([0x16, 0x16, 0x16, 0xff]));
const HEADER_COLOR: Color = Color(Rgba([0x4c, 0x54, 0x59, 0xff]));
const INPUT_COLOR: Color = Color(Rgba([0x2d, 0x31, 0x33, 0xff]));

const ICON_TRASH: &[u8] =
    include_bytes!("../../../../assets/fonts/FontAwesome/svgs/regular/trash-can.svg");
const ICON_ARROW_RIGHT: &[u8] =
    include_bytes!("../../../../assets/fonts/FontAwesome/svgs/solid/angle-right.svg");
const ICON_ARROW_DOWN: &[u8] =
    include_bytes!("../../../../assets/fonts/FontAwesome/svgs/solid/angle-down.svg");

#[derive(Clone, Debug)]
pub struct ComponentsPanel {
    pub state: Arc<Mutex<SceneState>>,
    pub writer: mpsc::Sender<Event>,
    pub modules: Modules,
}

impl Widget for ComponentsPanel {
    fn mount<T>(self, parent: &Context<T>) -> Context<()> {
        let style = Style {
            background: Background::Color(PANEL_COLOR.0),
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

        ComponentWrapper {
            id,
            name: name.to_owned(),
            writer: writer.clone(),
            component: component.clone(),
            descriptor,
        }
        .mount(&component_container);
    }

    let mut components = Vec::new();
    for module in modules.iter() {
        for record in module.records.iter() {
            if record.kind != RecordKind::COMPONENT {
                continue;
            }

            let descriptor = ComponentDescriptor::from_bytes(&record.data);
            components.push((
                RecordReference {
                    module: module.id,
                    record: record.id,
                },
                record.name.clone(),
                descriptor,
            ));
        }
    }

    if state.entities.iter().any(|e| e.is_selected) {
        mount_new_component_selector(&root, components, writer);
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

fn display_value<T, F>(ctx: &Context<()>, color: Color, label: &str, value: T, on_change: F)
where
    T: Display + FromStr + 'static,
    F: Into<Callback<T>>,
{
    let on_change = on_change.into();

    let root = Container::new()
        .style(Style {
            direction: Direction::Column,
            padding: Padding::splat(Size::Pixels(5)),
            ..Default::default()
        })
        .mount(ctx);

    let color_box = Container::new()
        .style(Style {
            background: Background::Color(color.0),
            // growth: Growth::y(1.0),
            ..Default::default()
        })
        .mount(&root);
    Text::new(label).mount(&color_box);

    Input::new()
        .value(value.to_string())
        .style(Style {
            background: Background::Color(INPUT_COLOR.0),
            padding: Padding::splat(Size::Pixels(1)),
            border_radius: BorderRadius::splat(Size::Pixels(5)),
            ..Default::default()
        })
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

struct ComponentWrapper {
    id: RecordReference,
    name: String,
    descriptor: ComponentDescriptor,
    writer: mpsc::Sender<Event>,
    component: RawComponent,
}

impl Widget for ComponentWrapper {
    fn mount<T>(self, parent: &Context<T>) -> Context<()> {
        let root = Container::new().mount(parent);

        let header = Container::new()
            .style(Style {
                direction: Direction::Column,
                padding: Padding::splat(Size::Pixels(5)),
                background: Background::Color(HEADER_COLOR.0),
                ..Default::default()
            })
            .mount(&root);
        let body = Container::new().mount(&root);

        let mut is_active = false;
        let collapse_button_id = Arc::new(Mutex::new(None));

        let writer = self.writer.clone();
        let collapse_button_id2 = collapse_button_id.clone();
        let on_collapse = move |()| {
            is_active ^= true;
            body.clear_children();

            let lock = collapse_button_id2.lock();
            let collapse_button_id: &Context<()> = lock.as_ref().unwrap();
            collapse_button_id.clear_children();

            if is_active {
                render_component(&body, self.id, &self.descriptor, &writer, &self.component);
            }

            let icon = if is_active {
                ICON_ARROW_DOWN
            } else {
                ICON_ARROW_RIGHT
            };

            Svg::new(SvgData::from_bytes(icon).unwrap(), 32, 32).mount(&collapse_button_id);
        };

        let collapse_button = Button::new()
            .style(Style {
                padding: Padding::splat(Size::Pixels(3)),
                ..Default::default()
            })
            .on_click(on_collapse)
            .mount(&header);
        Svg::new(SvgData::from_bytes(ICON_ARROW_RIGHT).unwrap(), 32, 32).mount(&collapse_button);
        *collapse_button_id.lock() = Some(collapse_button);

        Text::new(self.name).size(32.0).mount(&header);

        let on_delete = move |()| {
            self.writer.send(Event::DeleteComponent(self.id)).unwrap();
        };

        let delete_button = Button::new()
            .style(Style {
                padding: Padding::splat(Size::Pixels(3)),
                ..Default::default()
            })
            .on_click(on_delete)
            .mount(&header);
        Svg::new(SvgData::from_bytes(ICON_TRASH).unwrap(), 32, 32).mount(&delete_button);

        root
    }
}

fn render_component(
    ctx: &Context<()>,
    id: RecordReference,
    descriptor: &ComponentDescriptor,
    writer: &mpsc::Sender<Event>,
    component: &RawComponent,
) {
    let mut queue = VecDeque::new();

    for index in descriptor.root() {
        let field = descriptor.get(*index).unwrap();
        queue.push_back((ctx.clone(), field));
    }

    render_fields(id, descriptor, queue, writer, component);
}

fn render_fields<'a>(
    id: RecordReference,
    descriptor: &'a ComponentDescriptor,
    mut queue: VecDeque<(Context<()>, &'a Field)>,
    writer: &mpsc::Sender<Event>,
    component: &RawComponent,
) {
    let mut offset = 0;

    // If every input field gets a direct clone of the component
    // at the time of creation of the panel they cannot track changes
    // of other fields. The changes to fields would then overwrite
    // each other.
    // To prevent this we give every input field access to the same
    // shared component instance.
    let component = Arc::new(Mutex::new(component.clone()));

    while let Some((parent, field)) = queue.pop_front() {
        match &field.kind {
            FieldKind::Int(val) => {
                let field_len = usize::from(val.bits) / 8;
                let bits = val.bits;
                let is_signed = val.is_signed;

                let value = {
                    let component = component.lock();
                    let bytes = &component.as_bytes()[offset..offset + field_len];

                    match (bits, is_signed) {
                        (8, false) => u8::from_le_bytes(bytes.try_into().unwrap()) as i64,
                        (8, true) => i8::from_le_bytes(bytes.try_into().unwrap()) as i64,
                        (16, false) => u16::from_le_bytes(bytes.try_into().unwrap()) as i64,
                        (16, true) => u16::from_le_bytes(bytes.try_into().unwrap()) as i64,
                        (32, false) => u32::from_le_bytes(bytes.try_into().unwrap()) as i64,
                        (32, true) => i32::from_le_bytes(bytes.try_into().unwrap()) as i64,
                        (64, false) => u64::from_le_bytes(bytes.try_into().unwrap()) as i64,
                        (64, true) => i64::from_le_bytes(bytes.try_into().unwrap()),
                        _ => todo!(),
                    }
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
                display_value(&parent, color, &field.name, value, move |mut value: i64| {
                    let mut component = component.lock();

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

                    *component = RawComponent::new(bytes, fields);

                    writer
                        .send(Event::UpdateComponent(id, component.clone()))
                        .unwrap();
                });

                offset += field_len;
            }
            FieldKind::Float(val) => {
                let field_len = usize::from(val.bits) / 8;
                let bits = val.bits;

                let value = {
                    let component = component.lock();
                    let bytes = &component.as_bytes()[offset..offset + field_len];

                    match bits {
                        32 => f32::from_le_bytes(bytes.try_into().unwrap()) as f64,
                        64 => f64::from_le_bytes(bytes.try_into().unwrap()),
                        _ => todo!(),
                    }
                };

                let component = component.clone();
                let writer = writer.clone();
                display_value(
                    &parent,
                    COLOR_X,
                    &field.name,
                    FormatFloat(value),
                    move |FormatFloat(value): FormatFloat| {
                        let mut component = component.lock();

                        let mut bytes = component.as_bytes().to_vec();
                        let fields = component.fields().to_vec();

                        match bits {
                            32 => {
                                bytes[offset..offset + field_len]
                                    .copy_from_slice(&(value as f32).to_le_bytes());
                            }
                            64 => {
                                bytes[offset..offset + field_len]
                                    .copy_from_slice(&value.to_le_bytes());
                            }
                            _ => todo!(),
                        }

                        *component = RawComponent::new(bytes, fields);

                        writer
                            .send(Event::UpdateComponent(id, component.clone()))
                            .unwrap();
                    },
                );

                offset += field_len;
            }
            FieldKind::Struct(val) => {
                let root = Container::new().mount(&parent);
                Text::new(field.name.clone()).mount(&parent);

                for index in val.iter().rev() {
                    let field = descriptor.get(*index).unwrap();
                    queue.push_front((root.clone(), field));
                }
            }
            FieldKind::String(_) => todo!(),
            FieldKind::Enum(enum_field) => {
                let mut active_variant = {
                    let component = component.lock();

                    let bytes =
                        &component.as_bytes()[offset..offset + enum_field.tag_bits as usize / 8];

                    match bytes.len() {
                        1 => bytes[0] as u64,
                        2 => u16::from_le_bytes(bytes.try_into().unwrap()) as u64,
                        4 => u32::from_le_bytes(bytes.try_into().unwrap()) as u64,
                        8 => u64::from_le_bytes(bytes.try_into().unwrap()),
                        _ => todo!(),
                    }
                };

                let options = enum_field.variants.iter().map(|v| v.name.clone()).collect();

                let root = Container::new().mount(&parent);
                Text::new(field.name.clone()).mount(&root);

                let children_ctx = Arc::new(Mutex::new(None));

                Selection {
                    options,
                    on_change: Callback::from({
                        let children_ctx = children_ctx.clone();
                        let descriptor = descriptor.clone();
                        let writer = writer.clone();
                        let component = component.clone();
                        let enum_field = enum_field.clone();
                        move |index| {
                            let variant: &EnumFieldVariant = &enum_field.variants[index];

                            if variant.tag == active_variant {
                                return;
                            }

                            active_variant = variant.tag;

                            let children_ctx = children_ctx.lock();
                            let children_ctx: &Context<()> = children_ctx.as_ref().unwrap();
                            children_ctx.clear_children();

                            let mut queue = VecDeque::new();

                            for index in enum_field
                                .variant(active_variant)
                                .unwrap()
                                .fields
                                .iter()
                                .rev()
                            {
                                let field = descriptor.get(*index).unwrap();
                                queue.push_front((children_ctx.clone(), field));
                            }

                            let component = component.lock().clone();
                            render_fields(id, &descriptor, queue, &writer, &component);
                        }
                    }),
                }
                .mount(&root);

                let children = Container::new().mount(&root);
                *children_ctx.lock() = Some(children.clone());

                for index in enum_field
                    .variant(active_variant)
                    .unwrap()
                    .fields
                    .iter()
                    .rev()
                {
                    let field = descriptor.get(*index).unwrap();
                    queue.push_front((children.clone(), field));
                }

                offset += enum_field.tag_bits as usize / 8;
            }
        }
    }
}

#[derive(Clone, Debug)]
struct KeyValuePair<'a, T> {
    key: &'a str,
    value: T,
    on_change: Callback<T>,
}

impl<'a, T> Widget for KeyValuePair<'a, T>
where
    T: ToString + FromStr + 'static,
{
    fn mount<U>(self, parent: &Context<U>) -> Context<()> {
        let root = Container::new()
            .style(Style {
                direction: Direction::Column,
                padding: Padding::splat(Size::Pixels(5)),
                ..Default::default()
            })
            .mount(parent);

        Text::new(self.key).mount(&root);

        Input::new()
            .value(self.value)
            .style(Style {
                background: Background::Color(INPUT_COLOR.0),
                padding: Padding::splat(Size::Pixels(1)),
                border_radius: BorderRadius::splat(Size::Pixels(5)),
                ..Default::default()
            })
            .on_change(move |value: String| {
                if let Ok(value) = value.parse::<T>() {
                    self.on_change.call(value);
                }
            })
            .mount(&root);

        root
    }
}

struct FormatFloat(f64);

impl Display for FormatFloat {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:.3}", self.0)
    }
}

impl FromStr for FormatFloat {
    type Err = <f64 as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        f64::from_str(s).map(Self)
    }
}

fn mount_new_component_selector(
    cx: &Context<()>,
    components: Vec<(RecordReference, String, ComponentDescriptor)>,
    writer: &mpsc::Sender<Event>,
) {
    let options = components.iter().map(|(_, name, _)| name.clone()).collect();

    let writer = writer.clone();
    let on_change = Callback::from(move |index| {
        let (id, _, descriptor): &(RecordReference, String, ComponentDescriptor) =
            &components[index];
        let component = descriptor.default_component();

        writer.send(Event::UpdateComponent(*id, component)).unwrap();
    });

    Selection { options, on_change }.mount(cx);
}

struct EditComponentStorage {
    data: RawComponent,
    offsets: IndexMap<ComponentOffsetKey, usize>,
    next_offset: usize,
    next_key: usize,
}

impl EditComponentStorage {
    pub fn new(data: RawComponent) -> Self {
        Self {
            data,
            offsets: IndexMap::new(),
            next_offset: 0,
            next_key: 0,
        }
    }

    pub fn register(&mut self, size: usize) -> ComponentOffsetKey {
        let key = ComponentOffsetKey(self.next_key);
        self.next_key += 1;
        self.offsets.insert(key, self.next_offset);
        self.next_offset += size;
        key
    }

    pub fn extend(&self) {}

    pub fn get_mut(&mut self, key: ComponentOffsetKey) {}
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
struct ComponentOffsetKey(usize);
