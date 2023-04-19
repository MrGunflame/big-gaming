use std::collections::HashMap;
use std::time::Instant;

use bevy::prelude::{Plugin, Resource};
use game_common::entity::EntityId;
use game_common::module::ModuleId;
use game_common::world::world::WorldState;
use game_data::loader::FileLoader;
use game_data::record::RecordBody;
use game_script::plugin::ScriptPlugin;
use game_script::script::Script;
use game_script::scripts::Scripts;
use game_script::ScriptServer;
use tokio::runtime::Runtime;

pub struct ModulePlugin;

impl Plugin for ModulePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        let rt = Runtime::new().unwrap();

        let mut modules = Modules::new();
        let mut server = ScriptServer::new();
        let mut scripts = Scripts::new();

        rt.block_on(async {
            let mut dir = match tokio::fs::read_dir("./mods").await {
                Ok(dir) => dir,
                Err(err) => {
                    tracing::error!("failed to load modules found ./mods: {}", err);
                    std::process::exit(1);
                }
            };

            while let Some(entry) = dir.next_entry().await.unwrap() {
                let data = match FileLoader::load(entry.path()).await {
                    Ok(data) => data,
                    Err(err) => {
                        tracing::error!("cannot load {:?}: {}", entry.path(), err);
                        continue;
                    }
                };

                tracing::info!(
                    "loaded module {} ({})",
                    data.header.module.name,
                    data.header.module.id,
                );

                let mut records = Records::new();
                for record in data.records {
                    let mut world = WorldState::new();
                    world.insert(Instant::now());
                    let mut view = world.front_mut().unwrap();

                    match &record.body {
                        RecordBody::Action(action) => {
                            let handle =
                                server.insert(Script::load(&server, action.script.as_ref()));
                        }
                        _ => (),
                    }

                    records.insert(record);
                }

                modules.insert(ModuleData {
                    id: data.header.module.id,
                    records,
                });
            }
        });

        tracing::info!("loaded {} modules", modules.len());

        app.insert_resource(modules);
        app.insert_resource(server);
        app.add_plugin(ScriptPlugin);
    }
}

use self::records::Records;

pub mod records;

#[derive(Clone, Debug, Resource)]
pub struct Modules {
    modules: HashMap<ModuleId, ModuleData>,
}

impl Modules {
    pub fn new() -> Self {
        Self {
            modules: HashMap::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.modules.len()
    }

    pub fn get(&self, id: ModuleId) -> Option<&ModuleData> {
        self.modules.get(&id)
    }

    pub fn insert(&mut self, module: ModuleData) {
        self.modules.insert(module.id, module);
    }

    pub fn contains(&self, id: ModuleId) -> bool {
        self.modules.contains_key(&id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &ModuleData> {
        self.modules.values()
    }
}

#[derive(Clone, Debug)]
pub struct ModuleData {
    pub id: ModuleId,
    pub records: Records,
}
