use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Read;

use game_common::module::ModuleId;
use game_common::record::{RecordId, RecordReference};
use game_data::loader::FileLoader;
use game_data::record::{Record, RecordKind};
use game_data::DataBuffer;
use game_script::{Executor, RecordProvider};
use thiserror::Error;
use tokio::runtime::Runtime;

use crate::modules::core::load_core;

use self::records::Records;

mod core;
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

impl RecordProvider for Modules {
    fn get(&self, id: RecordReference) -> Option<&Record> {
        self.get(id.module)
            .map(|module| module.records.get(id.record))
            .flatten()
    }
}

#[derive(Clone, Debug)]
pub struct ModuleData {
    pub id: ModuleId,
    pub records: Records,
}

pub struct LoadResult {
    pub modules: Modules,
    pub executor: Executor,
}

pub fn load_modules() -> LoadResult {
    let mut modules = Modules::new();
    let mut executor = Executor::new();

    // Load the builtin core module.
    modules.insert(load_core());

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
                        load_module(data, &mut modules, &mut executor);
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

    LoadResult { modules, executor }
}

fn load_module(data: DataBuffer, modules: &mut Modules, executor: &mut Executor) {
    for script in data
        .records
        .iter()
        .filter(|r| r.kind == RecordKind::SCRIPT)
        .map(|r| &r.data)
    {
        let Ok(script) = std::str::from_utf8(&script) else {
            continue;
        };

        let buf = match (|| {
            let mut file = File::open(&script)?;

            let mut buf = Vec::new();
            file.read_to_end(&mut buf)?;
            Result::<_, std::io::Error>::Ok(buf)
        })() {
            Ok(buf) => buf,
            Err(err) => {
                tracing::error!(
                    "failed to load script from local path {:?}: {}",
                    script,
                    err,
                );

                continue;
            }
        };

        let handle = match executor.load(&buf) {
            Ok(script) => script,
            Err(err) => {
                tracing::error!(
                    "failed to load script from local path {:?}: {}",
                    script,
                    err,
                );

                continue;
            }
        };
    }

    let mut records = Records::new();
    for record in &data.records {
        // In case a linked asset is not present we still want to load
        // the record to not break linked records.
        records.insert(record.clone());
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
    use game_common::module::{Dependency, Module, ModuleId, ModuleIdExt, Version};
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
            version: Version::PLACEHOLDER,
            dependencies: vec![],
        });

        let dep2 = DataBuffer::new(Module {
            id: ModuleId::random(),
            name: String::from("Dependency 2"),
            version: Version::PLACEHOLDER,
            dependencies: vec![],
        });

        let module = DataBuffer::new(Module {
            id: ModuleId::random(),
            name: String::from("module"),
            version: Version::PLACEHOLDER,
            dependencies: vec![
                Dependency {
                    id: dep1.header.module.id,
                    name: Some(dep1.header.module.name.clone()),
                    version: dep1.header.module.version.clone(),
                },
                Dependency {
                    id: dep2.header.module.id,
                    name: Some(dep2.header.module.name.clone()),
                    version: dep2.header.module.version.clone(),
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
            version: Version::PLACEHOLDER,
            dependencies: vec![],
        });

        let dep2 = DataBuffer::new(Module {
            id: ModuleId::random(),
            name: String::from("Dependency 2"),
            version: Version::PLACEHOLDER,
            dependencies: vec![Dependency {
                id: dep1.header.module.id,
                name: Some(dep1.header.module.name.clone()),
                version: dep1.header.module.version.clone(),
            }],
        });

        let module = DataBuffer::new(Module {
            id: ModuleId::random(),
            name: String::from("module"),
            version: Version::PLACEHOLDER,
            dependencies: vec![Dependency {
                id: dep2.header.module.id,
                name: Some(dep2.header.module.name.clone()),
                version: dep2.header.module.version.clone(),
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
            version: Version::PLACEHOLDER,
            dependencies: vec![],
        });

        let module2 = DataBuffer::new(Module {
            id,
            name: String::from("module2"),
            version: Version::PLACEHOLDER,
            dependencies: vec![],
        });

        assert!(loader.load(module1).is_ok());
        assert_eq!(loader.load(module2).unwrap_err(), Error::Duplicate(id));
    }
}
