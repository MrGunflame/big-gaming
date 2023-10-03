use std::collections::{HashMap, HashSet};

use game_common::module::ModuleId;
use game_common::record::{RecordId, RecordReference};
use game_data::loader::FileLoader;
use game_data::record::{Record, RecordBody, RecordKind};
use game_data::DataBuffer;
use game_script::scripts::RecordTargets;
use game_script::ScriptServer;
use thiserror::Error;
use tokio::runtime::Runtime;

use self::records::Records;

pub mod records;

#[derive(Clone, Debug)]
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

    pub fn is_empty(&self) -> bool {
        self.len() == 0
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

impl Default for Modules {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug)]
pub struct ModuleData {
    pub id: ModuleId,
    pub records: Records,
}

pub struct LoadResult {
    pub modules: Modules,
    pub server: ScriptServer,
    pub record_targets: RecordTargets,
}

pub fn load_modules() -> LoadResult {
    let mut modules = Modules::new();
    let mut server = ScriptServer::new();
    let mut record_targets = RecordTargets::default();

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
                        load_module(data, &mut modules, &mut server, &mut record_targets);
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

    LoadResult {
        modules,
        server,
        record_targets,
    }
}

fn load_module(
    data: DataBuffer,
    modules: &mut Modules,
    server: &mut ScriptServer,
    record_targets: &mut RecordTargets,
) {
    let mut records = Records::new();
    for record in &data.records {
        // In case a linked asset is not present we still want to load
        // the record to not break linked records.
        records.insert(record.clone());

        for script in &record.scripts {
            let handle = match server.load(script.as_ref()) {
                Ok(script) => script,
                Err(err) => {
                    tracing::error!(
                        "failed to load script for record {} from local path {:?}: {}",
                        record.name,
                        script.as_ref(),
                        err,
                    );

                    continue;
                }
            };

            record_targets.push_script(
                RecordReference {
                    module: data.header.module.id,
                    record: record.id,
                },
                handle,
            );
        }

        match &record.body {
            RecordBody::Item(_) => {}
            RecordBody::Object(_) => {}
            RecordBody::Action(action) => {}
            RecordBody::Component(component) => {}
            RecordBody::Race(race) => {
                for action in &race.actions {
                    record_targets.push_action(
                        RecordReference {
                            module: data.header.module.id,
                            record: record.id,
                        },
                        *action,
                    );
                }
            }
        }
    }

    if let Err(err) = validate_module(modules, &data) {
        tracing::error!(
            "failed to load module {} ({}): {}",
            data.header.module.name,
            data.header.module.id,
            err,
        );

        return;
    }

    modules.insert(ModuleData {
        id: data.header.module.id,
        records,
    });
}

#[derive(Clone, Debug, Error)]
#[error("bad record {record}: {kind}")]
pub struct ValidationError {
    record: RecordId,
    kind: ValidationErrorKind,
}

#[derive(Clone, Debug, Error)]
pub enum ValidationErrorKind {
    /// A record linked an unknown [`ModuleId`].
    ///
    /// Note that any referenced records from external modules **MUST** be declared in an explicit
    /// depdency. A transitive dependency is not enough.
    #[error("unknown dependency: {0}")]
    UnknownDependency(ModuleId),
    /// A record linked to an unknown record inside a module.
    ///
    /// Note that an `UnknownRecord` means that the module was loaded successfully, but it did not
    /// contain the requested [`RecordId`].
    #[error("unknown record {id} in dependency {module}")]
    UnknownRecord { module: ModuleId, id: RecordId },
    #[error("invalid record kind {found:?}, expected {expected:?}")]
    InvalidKind {
        found: RecordKind,
        expected: RecordKind,
    },
}

fn validate_module(modules: &Modules, module: &DataBuffer) -> Result<(), ValidationError> {
    for record in &module.records {
        match &record.body {
            RecordBody::Action(_) => {}
            RecordBody::Component(_) => {}
            RecordBody::Item(item) => {
                for component in &item.components {
                    let module_id = component.record.module;

                    if module_id != module.header.module.id
                        && !module
                            .header
                            .module
                            .dependencies
                            .iter()
                            .any(|dep| dep.id == component.record.module)
                    {
                        return Err(ValidationError {
                            record: record.id,
                            kind: ValidationErrorKind::UnknownDependency(module_id),
                        });
                    }

                    match fetch_record(modules, module, component.record) {
                        Ok(rec) => {
                            if !rec.body.kind().is_component() {
                                return Err(ValidationError {
                                    record: record.id,
                                    kind: ValidationErrorKind::InvalidKind {
                                        found: rec.body.kind(),
                                        expected: RecordKind::Component,
                                    },
                                });
                            }
                        }
                        Err(err) => {
                            return Err(ValidationError {
                                record: record.id,
                                kind: err,
                            });
                        }
                    }
                }
            }
            RecordBody::Object(object) => {}
            RecordBody::Race(race) => {}
        }
    }

    Ok(())
}

/// Fetch a record from either the module itself, or any dependant modules.
fn fetch_record<'a>(
    modules: &'a Modules,
    module: &'a DataBuffer,
    id: RecordReference,
) -> Result<&'a Record, ValidationErrorKind> {
    if let Some(module) = modules.get(id.module) {
        if let Some(rec) = module.records.get(id.record) {
            return Ok(rec);
        } else {
            // Module loaded, but doesn't contain record.
            return Err(ValidationErrorKind::UnknownRecord {
                module: id.module,
                id: id.record,
            });
        }
    }

    if module.header.module.id != id.module {
        return Err(ValidationErrorKind::UnknownDependency(id.module));
    }

    module.records.iter().find(|rec| rec.id == id.record).ok_or(
        ValidationErrorKind::UnknownRecord {
            module: id.module,
            id: id.record,
        },
    )
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
