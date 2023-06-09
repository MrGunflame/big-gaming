use game_common::module::ModuleId;
use game_common::record::RecordId;
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
    let opts_string = opts.iter().map(|id| id.to_string()).collect();

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

    let fields = Fields { module_id, name };

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

        let record = Record {
            id: RecordId(0),
            name,
            body: RecordBody::Item(ItemRecord {
                mass: Default::default(),
                value: Default::default(),
                uri: Uri::new(),
                components: Default::default(),
                actions: Default::default(),
            }),
        };

        records.insert(module_id, record);
    })
}

struct Fields {
    module_id: ReadSignal<ModuleId>,
    name: ReadSignal<String>,
}
