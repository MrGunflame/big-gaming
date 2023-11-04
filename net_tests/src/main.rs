use game_common::module::ModuleId;
use game_common::record::RecordId;
use game_common::world::gen::stub::StubGenerator;
use game_core::counter::ManualInterval;
use game_core::modules::records::Records;
use game_core::modules::{ModuleData, Modules};
use game_data::components::objects::ObjectRecord;
use game_data::record::{Record, RecordBody};
use game_data::uri::Uri;
use game_script::scripts::RecordTargets;
use game_script::ScriptServer;
use game_server::config::Config;
use game_server::{GameServer, ServerState};

fn main() {
    let modules = load_test_modules();
    let config = Config {
        timestep: 60,
        player_streaming_source_distance: 2,
    };

    let interval = ManualInterval::new();

    let state = ServerState::new(
        StubGenerator.into(),
        modules,
        config,
        ScriptServer::new(),
        RecordTargets::default(),
    );
    let mut server = GameServer::new(state, interval);

    server.update();
}

const TEST_MODULE_ID: ModuleId = ModuleId::from_bytes([0; 16]);

fn load_test_modules() -> Modules {
    let mut records = Records::new();
    records.insert(Record {
        id: RecordId(1),
        name: "Test Object 01".to_owned(),
        scripts: vec![],
        components: vec![],
        body: RecordBody::Object(ObjectRecord {
            uri: Uri::new(),
            components: vec![],
        }),
    });

    let mut modules = Modules::new();
    modules.insert(ModuleData {
        id: TEST_MODULE_ID,
        records,
    });

    modules
}
