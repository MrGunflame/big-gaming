use ahash::HashMap;

use crate::record::RecordReference;

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

    pub fn len(&self) -> usize {
        self.components.len()
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

    pub fn iter(&self) -> Iter<'_> {
        Iter {
            inner: self.components.iter(),
        }
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

pub struct Iter<'a> {
    inner: std::collections::hash_map::Iter<'a, RecordReference, Component>,
}

impl<'a> Iterator for Iter<'a> {
    type Item = (RecordReference, &'a Component);

    fn next(&mut self) -> Option<Self::Item> {
        let (k, v) = self.inner.next()?;
        Some((*k, v))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len(), Some(self.len()))
    }
}

impl<'a> ExactSizeIterator for Iter<'a> {
    fn len(&self) -> usize {
        self.inner.len()
    }
}
