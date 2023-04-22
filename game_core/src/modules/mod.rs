use std::collections::{HashMap, HashSet};
use std::time::Instant;

use bevy::prelude::{App, Plugin, Resource};
use game_common::module::ModuleId;
use game_common::world::world::WorldState;
use game_data::loader::FileLoader;
use game_data::record::RecordBody;
use game_data::DataBuffer;
use game_script::plugin::ScriptPlugin;
use game_script::script::Script;
use game_script::ScriptServer;
use tokio::runtime::Runtime;

pub struct ModulePlugin;

impl Plugin for ModulePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.insert_resource(Modules::new());
        app.insert_resource(ScriptServer::new());

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

pub fn load_modules(app: &mut App) {
    let mut modules = Modules::new();
    let mut server = ScriptServer::new();

    let rt = Runtime::new().unwrap();

    let mut loader = ModuleLoader::new();

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

            match loader.load(data) {
                Ok(mods) => {
                    for data in mods {
                        load_module(data, &mut modules, &mut server);
                    }
                }
                Err(Error::Duplicate(id)) => {
                    tracing::error!("attempted to load module with duplicate id: {}", id);
                }
                // Module is queued
                Err(Error::Delayed) => (),
            }
        }
    });

    for data in loader.clear() {
        tracing::error!(
            "failed to load module {} ({})",
            data.header.module.name,
            data.header.module.id,
        );
    }

    tracing::info!("loaded {} modules", modules.len());

    app.insert_resource(modules);
    app.insert_resource(server);
}

fn load_module(data: DataBuffer, modules: &mut Modules, server: &mut ScriptServer) {
    let mut records = Records::new();
    for record in data.records {
        let mut world = WorldState::new();
        world.insert(Instant::now());
        match &record.body {
            RecordBody::Action(action) => {
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

/// Temporary store used while loading modules.
///
/// Loading modules follows the process of:
/// 1. attempt to load modules in the order first-come-first-served
/// 2. modules that cannot yet be loaded because they have deps are backlogged.
///
#[derive(Clone, Debug, Default)]
struct ModuleLoader {
    /// Modules that have been loaded.
    loaded: HashSet<ModuleId>,
    /// Modules that are queued to be loaded, but are missing dependencies.
    ///
    /// queued_module.id => (number_of_waiting_deps, queued_module)
    queued: HashMap<ModuleId, (usize, DataBuffer)>,
    ///
    ///
    /// requested_dependent_module.id => requesting_modules
    backlog: HashMap<ModuleId, Vec<ModuleId>>,
}

impl ModuleLoader {
    pub fn new() -> Self {
        Self {
            queued: HashMap::new(),
            loaded: HashSet::new(),
            backlog: HashMap::new(),
        }
    }

    pub fn load(&mut self, data: DataBuffer) -> Result<Vec<DataBuffer>, Error> {
        if self.loaded.contains(&data.header.module.id) {
            return Err(Error::Duplicate(data.header.module.id));
        }

        let mut num_deps = 0;
        for dep in &data.header.module.dependencies {
            if !self.loaded.contains(&dep.id) {
                num_deps += 1;

                self.backlog
                    .entry(dep.id)
                    .or_default()
                    .push(data.header.module.id);
            }
        }

        if num_deps == 0 {
            self.loaded.insert(data.header.module.id);

            let id = data.header.module.id;
            let mut loaded = vec![data];
            loaded.extend(self.load_queued_modules(id));

            Ok(loaded)
        } else {
            let (n, _) = self
                .queued
                .entry(data.header.module.id)
                .or_insert_with(|| (0, data));
            *n += num_deps;

            Err(Error::Delayed)
        }
    }

    fn load_queued_modules(&mut self, dependency: ModuleId) -> Vec<DataBuffer> {
        // Load recurisve.

        let mut load = vec![];

        let Some(dependents) = self.backlog.remove(&dependency) else {
            return vec![];
        };

        for id in dependents {
            let (num_deps, _) = self.queued.get_mut(&id).unwrap();

            *num_deps -= 1;

            // All required dependencies loading.
            if *num_deps == 0 {
                let (_, data) = self.queued.remove(&id).unwrap();

                self.loaded.insert(data.header.module.id);

                load.push(data);
            }
        }

        // Load all modules that depend on now loading modules.

        let mut index = 0;
        let len = load.len();

        while index < len {
            let id = load[index].header.module.id;

            load.extend(self.load_queued_modules(id));

            index += 1;
        }

        load
    }

    /// Clears the backlog, returning all modules that were unable to be loaded.
    ///
    pub fn clear(&mut self) -> Vec<DataBuffer> {
        vec![]
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Error {
    Duplicate(ModuleId),
    Delayed,
}

#[cfg(test)]
mod tests {
    use game_common::module::{Dependency, Module, ModuleId, Version};
    use game_data::DataBuffer;

    use super::records::Records;
    use super::{Error, ModuleData, ModuleLoader, Modules};

    #[test]
    fn test_modules() {
        let mut modules = Modules::new();
        modules.insert(ModuleData {
            id: ModuleId::CORE,
            records: Records::new(),
        });

        assert!(modules.get(ModuleId::CORE).is_some());
        assert!(modules.contains(ModuleId::CORE));
    }

    #[test]
    fn module_loader_no_dependencies() {
        let mut loader = ModuleLoader::new();
        let module = DataBuffer::new(Module::core());

        let res = loader.load(module).unwrap();

        assert_eq!(res.len(), 1);
        assert_eq!(res[0].header.module.id, ModuleId::CORE);
    }

    #[test]
    fn module_loader_flat_dependencies() {
        let mut loader = ModuleLoader::new();

        let dep1 = DataBuffer::new(Module {
            id: ModuleId::random(),
            name: String::from("Dependency 1"),
            version: Version,
            dependencies: vec![],
        });

        let dep2 = DataBuffer::new(Module {
            id: ModuleId::random(),
            name: String::from("Dependency 2"),
            version: Version,
            dependencies: vec![],
        });

        let module = DataBuffer::new(Module {
            id: ModuleId::random(),
            name: String::from("module"),
            version: Version,
            dependencies: vec![
                Dependency {
                    id: dep1.header.module.id,
                    name: Some(dep1.header.module.name.clone()),
                    version: dep1.header.module.version,
                },
                Dependency {
                    id: dep2.header.module.id,
                    name: Some(dep2.header.module.name.clone()),
                    version: dep2.header.module.version,
                },
            ],
        });

        assert_eq!(loader.load(module.clone()).unwrap_err(), Error::Delayed);

        let res1 = loader.load(dep1.clone()).unwrap();
        assert_eq!(res1.len(), 1);
        assert_eq!(res1[0].header.module.id, dep1.header.module.id);

        let res2 = loader.load(dep2.clone()).unwrap();
        assert_eq!(res2.len(), 2);
        assert_eq!(res2[0].header.module.id, dep2.header.module.id);
        assert_eq!(res2[1].header.module.id, module.header.module.id);
    }

    #[test]
    fn module_loader_nested_dependencies() {
        let mut loader = ModuleLoader::new();

        let dep1 = DataBuffer::new(Module {
            id: ModuleId::random(),
            name: String::from("Dependency 1"),
            version: Version,
            dependencies: vec![],
        });

        let dep2 = DataBuffer::new(Module {
            id: ModuleId::random(),
            name: String::from("Dependency 2"),
            version: Version,
            dependencies: vec![Dependency {
                id: dep1.header.module.id,
                name: Some(dep1.header.module.name.clone()),
                version: dep1.header.module.version,
            }],
        });

        let module = DataBuffer::new(Module {
            id: ModuleId::random(),
            name: String::from("module"),
            version: Version,
            dependencies: vec![Dependency {
                id: dep2.header.module.id,
                name: Some(dep2.header.module.name.clone()),
                version: dep2.header.module.version,
            }],
        });

        assert_eq!(loader.load(module.clone()).unwrap_err(), Error::Delayed);
        assert_eq!(loader.load(dep2.clone()).unwrap_err(), Error::Delayed);

        let res = loader.load(dep1.clone()).unwrap();
        assert_eq!(res.len(), 3);
        assert_eq!(res[0].header.module.id, dep1.header.module.id);
        assert_eq!(res[1].header.module.id, dep2.header.module.id);
        assert_eq!(res[2].header.module.id, module.header.module.id);
    }

    #[test]
    fn module_loader_duplicate() {
        let mut loader = ModuleLoader::new();

        let id = ModuleId::random();

        let module1 = DataBuffer::new(Module {
            id,
            name: String::from("module1"),
            version: Version,
            dependencies: vec![],
        });

        let module2 = DataBuffer::new(Module {
            id,
            name: String::from("module2"),
            version: Version,
            dependencies: vec![],
        });

        assert!(loader.load(module1).is_ok());
        assert_eq!(loader.load(module2).unwrap_err(), Error::Duplicate(id));
    }
}
