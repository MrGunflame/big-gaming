use game_common::module::ModuleId;
use game_common::record::RecordId;
use game_common::units::Mass;
use game_data::components::item::ItemRecord;
use game_data::record::{Record, RecordBody, RecordKind};
use game_data::uri::Uri;
use game_input::mouse::MouseButtonInput;
use game_ui::events::Context;
use game_ui::reactive::{create_signal, ReadSignal, Scope};
use game_ui::render::style::{
    Background, Bounds, Direction, Justify, Padding, Size, SizeVec2, Style,
};
use game_ui::{component, view};

use game_ui::widgets::*;

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
        _ => todo!(),
    };

    let fields = Fields {
        module_id,
        name,
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
        };

        let record = Record {
            id: RecordId(0),
            name,
            body,
        };

        records.insert(module_id, record);
    })
}

struct Fields {
    module_id: ReadSignal<ModuleId>,
    name: ReadSignal<String>,
    body: RecordBodyFields,
}

enum RecordBodyFields {
    Item(ItemFields),
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
