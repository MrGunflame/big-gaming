use std::path::PathBuf;

use game_common::module::ModuleId;
use game_common::record::RecordId;
use game_common::units::Mass;
use game_data::components::actions::ActionRecord;
use game_data::components::components::ComponentRecord;
use game_data::components::item::ItemRecord;
use game_data::components::objects::ObjectRecord;
use game_data::record::{Record, RecordBody, RecordKind};
use game_data::uri::Uri;
use game_input::mouse::MouseButtonInput;
use game_ui::events::Context;
use game_ui::reactive::{ReadSignal, Scope};
use game_ui::style::{Background, Bounds, Direction, Justify, Padding, Size, SizeVec2, Style};
use game_ui::widgets::{Button, Container, Input, ParseInput, Selection, Text, Widget};
use parking_lot::Mutex;

use crate::state::module::Modules;
use crate::state::record::Records;

pub struct EditRecord {
    pub record: Record,
    pub records: Records,
    pub modules: Modules,
    pub module_id: ModuleId,
}

impl Widget for EditRecord {
    fn build(self, cx: &Scope) -> Scope {
        let root = cx.append(Container::new().style(Style {
            padding: Padding::splat(Size::Pixels(5)),
            justify: Justify::SpaceBetween,
            ..Default::default()
        }));

        let record_id = self.record.id;
        let fields = render_record(
            &root,
            &self.modules,
            self.record.kind(),
            Some((self.module_id, self.record)),
        );

        let button = root.append(Button::new().on_click(create_record(
            self.records,
            fields,
            Some(record_id),
        )));
        button.append(Text::new().text("Ok".to_owned()));

        root
    }
}

pub struct CreateRecord {
    pub kind: RecordKind,
    pub records: Records,
    pub modules: Modules,
}

impl Widget for CreateRecord {
    fn build(self, cx: &Scope) -> Scope {
        let root = cx.append(Container::new().style(Style {
            padding: Padding::splat(Size::Pixels(5)),
            justify: Justify::SpaceBetween,
            ..Default::default()
        }));

        let fields = render_record(&root, &self.modules, self.kind, None);

        let button = root.append(Button::new().on_click(create_record(self.records, fields, None)));
        button.append(Text::new().text("Ok".to_owned()));

        root
    }
}

fn render_record(
    root: &Scope,
    modules: &Modules,
    kind: RecordKind,
    record: Option<(ModuleId, Record)>,
) -> Fields {
    let (module_id, set_module_id) = {
        let value = match &record {
            Some((module_id, _)) => *module_id,
            None => ModuleId::CORE,
        };

        root.create_signal(value)
    };

    let (name, set_name) = {
        let value = match &record {
            Some((_, record)) => record.name.clone(),
            None => String::new(),
        };

        root.create_signal(value)
    };

    let metadata = root.append(Container::new().style(Style {
        direction: Direction::Column,
        ..Default::default()
    }));

    let name_col = metadata.append(Container::new());
    let val_col = metadata.append(Container::new());

    for text in ["Module", "ID", "Name"] {
        name_col.append(Text::new().text(text.to_owned()));
    }

    let opts: Vec<ModuleId> = modules.iter().map(|m| m.module.id).collect();
    let opts_string = modules
        .iter()
        .map(|m| format!("{} ({})", m.module.name, m.module.id))
        .collect();

    let on_change = move |index| {
        let id = opts[index];

        set_module_id.update(|val| *val = id);
    };

    val_col.append(Selection::new().options(opts_string).on_change(on_change));
    val_col.append(Text::new().text("TODO".to_owned()));

    let style = Style {
        bounds: Bounds {
            min: SizeVec2 {
                x: Size::Pixels(100),
                y: Size::Pixels(20),
            },
            ..Default::default()
        },
        background: Background::GRAY,
        ..Default::default()
    };

    val_col.append(
        Input::new()
            .value(name.get_untracked())
            .style(style)
            .on_change(move |s| set_name.set(s)),
    );

    let body = match &record {
        Some((_, record)) => match &record.body {
            RecordBody::Item(item) => {
                RecordBodyFields::Item(render_item(&root, Some(item.clone())))
            }
            RecordBody::Action(action) => RecordBodyFields::Action,
            RecordBody::Component(component) => RecordBodyFields::Component,
            RecordBody::Object(object) => {
                RecordBodyFields::Object(render_object(&root, Some(object.clone())))
            }
            RecordBody::Race(race) => todo!(),
        },
        None => match kind {
            RecordKind::Item => RecordBodyFields::Item(render_item(&root, None)),
            RecordKind::Action => RecordBodyFields::Action,
            RecordKind::Component => RecordBodyFields::Component,
            RecordKind::Object => RecordBodyFields::Object(render_object(&root, None)),
            RecordKind::Race => todo!(),
        },
    };

    let scripts = render_script_section(&root);

    Fields {
        module_id,
        name,
        scripts,
        body,
    }
}

fn create_record(
    records: Records,
    fields: Fields,
    // Record id if updating.
    record_id: Option<RecordId>,
) -> Box<dyn Fn(Context<MouseButtonInput>) + Send + Sync + 'static> {
    Box::new(move |ctx| {
        let module_id = fields.module_id.get_untracked();
        let name = fields.name.get_untracked();

        // Use `ModuleId::CORE` as a placeholder/`None` value.
        if module_id == ModuleId::CORE {
            return;
        }

        ctx.window.close();

        let scripts = fields
            .scripts
            .get_untracked()
            .into_iter()
            .map(|s| Uri::from(PathBuf::from(s)))
            .collect();

        let body = match &fields.body {
            RecordBodyFields::Item(item) => {
                let value = item.value.get_untracked();
                let mass = item.mass.get_untracked();
                let scene = item.scene.get_untracked();

                RecordBody::Item(ItemRecord {
                    mass,
                    value,
                    scene: Uri::from(PathBuf::from(scene)),
                    components: Default::default(),
                    actions: Default::default(),
                })
            }
            RecordBodyFields::Action => RecordBody::Action(ActionRecord {
                description: String::new(),
            }),
            RecordBodyFields::Component => RecordBody::Component(ComponentRecord {
                description: String::new(),
            }),
            RecordBodyFields::Object(object) => {
                let model = object.model.get_untracked();

                RecordBody::Object(ObjectRecord {
                    uri: Uri::from(PathBuf::from(model)),
                    components: Default::default(),
                })
            }
        };

        match record_id {
            Some(id) => {
                let record = Record {
                    id,
                    name,
                    scripts,
                    body,
                    components: vec![],
                };

                records.update(module_id, record);
            }
            None => {
                let id = records.take_id(module_id);

                let record = Record {
                    id,
                    name,
                    scripts,
                    body,
                    components: vec![],
                };

                records.insert(module_id, record);
            }
        }
    })
}

struct Fields {
    module_id: ReadSignal<ModuleId>,
    name: ReadSignal<String>,
    scripts: ReadSignal<Vec<String>>,
    body: RecordBodyFields,
}

enum RecordBodyFields {
    Item(ItemFields),
    Action,
    Component,
    Object(ObjectFields),
}

struct ItemFields {
    mass: ReadSignal<Mass>,
    value: ReadSignal<u64>,
    scene: ReadSignal<String>,
}

fn render_item(cx: &Scope, item: Option<ItemRecord>) -> ItemFields {
    let (value, set_value) = {
        let value = match &item {
            Some(item) => item.value,
            None => 0,
        };

        cx.create_signal(value)
    };

    let (mass, set_mass) = {
        let value = match &item {
            Some(item) => item.mass,
            None => Mass::default(),
        };

        cx.create_signal(value)
    };

    let (scene, set_scene) = {
        let value = match &item {
            Some(item) => item.scene.as_ref().to_string_lossy().to_string(),
            None => String::new(),
        };

        cx.create_signal(value)
    };

    let item = cx.append(Container::new().style(Style {
        direction: Direction::Column,
        ..Default::default()
    }));

    let name_col = item.append(Container::new());
    let val_col = item.append(Container::new());

    let style = Style {
        bounds: Bounds {
            min: SizeVec2::splat(Size::Pixels(20)),
            ..Default::default()
        },
        background: Background::GRAY,
        ..Default::default()
    };

    // Value
    name_col.append(Text::new().text("Value".to_owned()));
    val_col.append(
        ParseInput::new(value.get_untracked())
            .style(style.clone())
            .on_change(move |val| set_value.set(val)),
    );

    // Mass
    name_col.append(Text::new().text("Mass".to_owned()));
    val_col.append(
        ParseInput::new(mass.get_untracked().to_grams())
            .style(style.clone())
            .on_change(move |val| set_mass.set(Mass::from_grams(val))),
    );

    // Model
    name_col.append(Text::new().text("Model".to_owned()));
    val_col.append(
        ParseInput::new(scene.get_untracked())
            .style(style)
            .on_change(move |val| set_scene.set(val)),
    );

    ItemFields { mass, value, scene }
}

struct ObjectFields {
    model: ReadSignal<String>,
}

fn render_object(cx: &Scope, object: Option<ObjectRecord>) -> ObjectFields {
    let (model, set_model) = {
        let value = match object {
            Some(object) => object.uri.as_ref().to_string_lossy().to_string(),
            None => String::new(),
        };

        cx.create_signal(value)
    };

    let root = cx.append(Container::new().style(Style {
        direction: Direction::Column,
        ..Default::default()
    }));

    let name_col = root.append(Container::new());
    let val_col = root.append(Container::new());

    let style = Style {
        bounds: Bounds {
            min: SizeVec2::splat(Size::Pixels(20)),
            ..Default::default()
        },
        background: Background::GRAY,
        ..Default::default()
    };

    // Model
    name_col.append(Text::new().text("Model".to_owned()));
    val_col.append(
        Input::new()
            .value(model.get_untracked())
            .style(style)
            .on_change(move |val| set_model.set(val)),
    );

    ObjectFields { model }
}

fn render_script_section(cx: &Scope) -> ReadSignal<Vec<String>> {
    let (scripts, set_scripts) = cx.create_signal(Vec::<String>::new());

    let root = cx.append(Container::new());

    let script_list = cx.append(Container::new());

    {
        let scripts = scripts.clone();
        let id = Mutex::new(None);
        let cx2 = cx.clone();
        cx.create_effect(move || {
            let id = &mut *id.lock();
            if let Some(id) = id {
                cx2.remove(*id);
            }

            let root = cx2.append(Container::new());
            *id = root.id();

            let scripts = scripts.get();

            for script in scripts {
                root.append(Text::new().text(script));
            }
        });
    }

    let (new_script, set_new_script) = cx.create_signal(String::new());

    let on_click = move |_| {
        set_scripts.update(|v| {
            v.push(new_script.get_untracked());
        });
    };

    let style = Style {
        bounds: Bounds {
            min: SizeVec2 {
                x: Size::Pixels(100),
                y: Size::Pixels(20),
            },
            ..Default::default()
        },
        background: Background::GRAY,
        ..Default::default()
    };

    root.append(
        Input::new()
            .style(style)
            .on_change(move |s| set_new_script.set(s)),
    );

    let button = root.append(Button::new().on_click(on_click));
    button.append(Text::new().text("New Script".to_owned()));

    scripts
}
