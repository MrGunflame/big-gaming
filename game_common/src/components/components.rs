use ahash::HashMap;
use bytemuck::{Pod, Zeroable};

use crate::module::ModuleId;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Components {
    components: HashMap<RecordReference, Component>,
}

impl Components {
    pub fn new() -> Self {
        Self {
            components: HashMap::default(),
        }
    }

    pub fn insert(&mut self, r: RecordReference, comp: Component) {
        self.components.insert(r, comp);
    }

    pub fn get(&self, r: RecordReference) -> Option<&Component> {
        self.components.get(&r)
    }

    pub fn get_mut(&mut self, r: RecordReference) -> Option<&mut Component> {
        self.components.get_mut(&r)
    }

    pub fn remove(&mut self, r: RecordReference) {
        self.components.remove(&r);
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Component {
    I32(i32),
    I64(i64),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Zeroable, Pod)]
#[repr(C)]
pub struct RecordReference {
    pub module: ModuleId,
    pub record: u32,
}
