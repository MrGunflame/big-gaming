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
use game_ui::reactive::{create_effect, create_signal, ReadSignal, Scope};
use game_ui::render::style::{
    Background, Bounds, Direction, Justify, Padding, Size, SizeVec2, Style,
};
use game_ui::{component, view};

use game_ui::widgets::*;
use parking_lot::Mutex;

use crate::state::module::Modules;
use crate::state::record::Records;

#[component]
pub fn CreateRecord(cx: &Scope, kind: RecordKind, records: Records, modules: Modules) -> Scope {
    let (module_id, set_module_id) = create_signal(cx, ModuleId::CORE);
    let (name, set_name) = create_signal(cx, String::new());

    let root = view! {
        cx,
        <Container style={Style{ padding: Padding::splat(Size::Pixels(5.0)), justify: Justify::SpaceBetween, ..Default::default() }}>
        </Container>
    };

    let metadata = view! {
        root,
        <Container style={Style { direction: Direction::Column, ..Default::default() }}>
        </Container>
    };

    let name_col = view! {
        metadata,
        <Container style={Style::default()}>
        </Container>
    };

    for text in ["Module", "ID", "Name"] {
        view! {
            name_col,
            <Text text={text.into()}>
            </Text>
        };
    }

    let val_col = view! {
        metadata,
        <Container style={Style::default()}>
        </Container>
    };

    let opts: Vec<ModuleId> = modules.iter().map(|m| m.module.id).collect();
    let opts_string = modules
        .iter()
        .map(|m| format!("{} ({})", m.module.name, m.module.id))
        .collect();

    let on_change = move |index| {
        let id = opts[index];

        set_module_id.update(|val| *val = id);
    };

    view! {
        val_col,
        <Selection value={None} options={opts_string} on_change={on_change.into()}>
        </Selection>
    };

    view! {
        val_col,
        <Text text={"TODO".into()}>
        </Text>
    };

    let style = Style {
        bounds: Bounds {
            min: SizeVec2 {
                x: Size::Pixels(100.0),
                y: Size::Pixels(20.0),
            },
            ..Default::default()
        },
        background: Background::GRAY,
        ..Default::default()
    };

    view! {
        val_col,
        <Input value={name.get_untracked()} on_change={set_name.into()} style={style}>
        </Input>
    };

    let body = match kind {
        RecordKind::Item => RecordBodyFields::Item(render_item(&root)),
        RecordKind::Action => RecordBodyFields::Action,
        RecordKind::Component => RecordBodyFields::Component,
        RecordKind::Object => RecordBodyFields::Object(render_object(&root)),
    };

    let scripts = render_script_section(&root);

    let fields = Fields {
        module_id,
        name,
        scripts,
        body,
    };

    view! {
        root,
        <Button style={Style::default()} on_click={create_record(records, fields).into()}>
            <Text text={"OK".into()}>
            </Text>
        </Button>
    };

    root
}

fn create_record(
    records: Records,
    fields: Fields,
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

                RecordBody::Item(ItemRecord {
                    mass,
                    value,
                    uri: Uri::new(),
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

        let record = Record {
            id: RecordId(0),
            name,
            scripts,
            body,
        };

        records.insert(module_id, record);
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
}

fn render_item(cx: &Scope) -> ItemFields {
    let (value, set_value) = create_signal(cx, 0);
    let (mass, set_mass) = create_signal(cx, Mass::default());

    let item = view! {
        cx,
        <Container style={Style::default()}>
        </Container>
    };

    let name_col = view! {
        item,
        <Container style={Style::default()}>
        </Container>
    };

    for text in ["Value", "Mass"] {
        view! {
            name_col,
            <Text text={text.into()}>
            </Text>
        };
    }

    let val_col = view! {
        item,
        <Container style={Style::default()}>
        </Container>
    };

    let value_change = move |s: String| {
        if let Ok(val) = s.parse() {
            set_value.update(|v| *v = val);
        }
    };

    view! {
        val_col,
        <Input value={value.get_untracked().to_string()} style={Style::default()} on_change={value_change.into()}>
        </Input>
    };

    let mass_change = move |s: String| {
        if let Ok(val) = s.parse() {
            set_mass.update(|v| *v = Mass::from_grams(val));
        }
    };

    view! {
        val_col,
        <Input value={mass.get_untracked().to_grams().to_string()} style={Style::default()} on_change={mass_change.into()}>
        </Input>
    };

    ItemFields { mass, value }
}

struct ObjectFields {
    model: ReadSignal<String>,
}

fn render_object(cx: &Scope) -> ObjectFields {
    let (model, set_model) = create_signal(cx, String::new());

    let root = view! {
        cx,
        <Container style={Style::default()}>
        </Container>
    };

    let name_col = view! {
        root,
        <Container style={Style::default()}>
        </Container>
    };

    for text in ["Model"] {
        view! {
            name_col,
            <Text text={text.into()}>
            </Text>
        };
    }

    let val_col = view! {
        root,
        <Container style={Style::default()}>
        </Container>
    };

    let on_change = move |s: String| {
        set_model.set(s);
    };

    view! {
        val_col,
        <Input value={model.get_untracked().to_string()} style={Style::default()} on_change={on_change.into()}>
        </Input>
    };

    ObjectFields { model }
}

fn render_script_section(cx: &Scope) -> ReadSignal<Vec<String>> {
    let (scripts, set_scripts) = create_signal(cx, Vec::<String>::new());

    let root = view! {
        cx,
        <Container style={Style::default()}>
        </Container>
    };

    let script_list = view! {
        root,
        <Container style={Style::default()}>
        </Container>
    };

    {
        let scripts = scripts.clone();
        let id = Mutex::new(None);
        let cx2 = cx.clone();
        create_effect(cx, move |_| {
            let id = &mut *id.lock();
            if let Some(id) = id {
                cx2.remove(*id);
            }

            let root = view! {
                script_list,
                <Container style={Style::default()}>
                </Container>
            };
            *id = root.id();

            let scripts = scripts.get();

            for script in scripts {
                view! {
                    root,
                    <Text text={script.into()}>
                    </Text>
                };
            }
        });
    }

    let (new_script, set_new_script) = create_signal(cx, String::new());

    let on_change = move |s| {
        set_new_script.set(s);
    };

    let on_click = move |_| {
        set_scripts.update(|v| {
            v.push(new_script.get_untracked());
        });
    };

    let style = Style {
        bounds: Bounds {
            min: SizeVec2 {
                x: Size::Pixels(100.0),
                y: Size::Pixels(20.0),
            },
            ..Default::default()
        },
        background: Background::GRAY,
        ..Default::default()
    };

    view! {
        root,
        <Input style={style} value={"".to_owned()} on_change={on_change.into()}>
        </Input>
    };

    view! {
        root,
        <Button style={Style::default()} on_click={on_click.into()}>
            <Text text={"New Script".into()}>
            </Text>
        </Button>
    };

    scripts
}
