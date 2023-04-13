use std::collections::HashMap;

use bevy::prelude::{Plugin, Resource};
use game_common::module::ModuleId;
use game_data::loader::FileLoader;
use tokio::runtime::Runtime;

pub struct ModulePlugin;

impl Plugin for ModulePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        let rt = Runtime::new().unwrap();

        let mut modules = Modules::new();

        rt.block_on(async {
            let mut dir = match tokio::fs::read_dir("./mods").await {
                Ok(dir) => dir,
                Err(err) => {
                    tracing::error!("failed to load modules: {}", err);
                    std::process::exit(1);
                }
            };

            while let Some(entry) = dir.next_entry().await.unwrap() {
                let data = FileLoader::load(entry.path()).await.unwrap();

                let mut records = Records::new();
                for record in data.records {
                    records.insert(record);
                }

                modules.insert(ModuleData {
                    id: data.header.module.id,
                    records,
                });
            }
        });

        app.insert_resource(modules);
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

    pub fn get(&self, id: ModuleId) -> Option<&ModuleData> {
        self.modules.get(&id)
    }

    pub fn insert(&mut self, module: ModuleData) {
        self.modules.insert(module.id, module);
    }

    pub fn contains(&self, id: ModuleId) -> bool {
        self.modules.contains_key(&id)
    }
}

#[derive(Clone, Debug)]
pub struct ModuleData {
    pub id: ModuleId,
    pub records: Records,
}
