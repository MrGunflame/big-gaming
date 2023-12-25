pub use game_wasm::record::ModuleId;

#[derive(Clone, Debug)]
pub struct Module {
    pub id: ModuleId,
    pub name: String,
    pub version: Version,
    pub dependencies: Vec<Dependency>,
}

impl Module {
    // FIXME: Change this to constant if possible.
    pub fn core() -> Self {
        Self {
            id: ModuleId::CORE,
            name: String::from("core"),
            version: Version,
            dependencies: Vec::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Dependency {
    pub id: ModuleId,
    pub name: Option<String>,
    //TODO
    pub version: Version,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Version;

pub trait ModuleIdExt {
    fn random() -> Self;
}

impl ModuleIdExt for ModuleId {
    fn random() -> Self {
        let uuid = uuid::Uuid::new_v4();
        Self::from_bytes(uuid.into_bytes())
    }
}
