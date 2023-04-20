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
pub struct Component {
    pub bytes: Vec<u8>,
}

impl Component {
    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    pub fn as_ptr(&self) -> *const u8 {
        self.bytes.as_ptr()
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Zeroable, Pod)]
#[repr(C)]
pub struct RecordReference {
    pub module: ModuleId,
    pub record: u32,
}
