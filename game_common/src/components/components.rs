use std::borrow::Cow;
use std::sync::Arc;

use ahash::HashMap;
use game_wasm::encoding::{BinaryReader, Field, Primitive};
use thiserror::Error;

use crate::entity::EntityId;
use crate::record::RecordReference;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Components {
    components: HashMap<RecordReference, RawComponent>,
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

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn insert(&mut self, r: RecordReference, comp: RawComponent) {
        self.components.insert(r, comp);
    }

    pub fn get(&self, r: RecordReference) -> Option<&RawComponent> {
        self.components.get(&r)
    }

    pub fn get_mut(&mut self, r: RecordReference) -> Option<&mut RawComponent> {
        self.components.get_mut(&r)
    }

    pub fn remove(&mut self, r: RecordReference) -> Option<RawComponent> {
        self.components.remove(&r)
    }

    pub fn iter(&self) -> Iter<'_> {
        Iter {
            inner: self.components.iter(),
        }
    }

    pub fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut(RecordReference, &mut RawComponent) -> bool,
    {
        self.components.retain(|k, v| f(*k, v));
    }

    /// Returns the `Components` intersecting with both `self` and `other`, i.e. the `Components`
    /// that are contained in both `self` and `other`. If a component exists the component from
    /// `self` is chosen.
    pub fn intersection(&self, other: &Self) -> Components {
        let mut components = self.components.clone();

        for id in self.components.keys() {
            if !other.components.contains_key(id) {
                components.remove(id);
            }
        }

        Self { components }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawComponent {
    bytes: Arc<[u8]>,
    fields: Arc<[Field]>,
}

impl RawComponent {
    pub fn new<T>(bytes: T, fields: impl Into<Arc<[Field]>>) -> Self
    where
        T: Into<Arc<[u8]>>,
    {
        Self {
            bytes: bytes.into(),
            fields: fields.into(),
        }
    }

    pub fn fields(&self) -> &[Field] {
        &self.fields
    }

    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn as_ptr(&self) -> *const u8 {
        self.bytes.as_ptr()
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn reader(&self) -> BinaryReader {
        BinaryReader::new(self.bytes.to_vec(), self.fields.to_vec().into())
    }

    pub fn remap(
        self,
        mut get_entity: impl FnMut(EntityId) -> Option<EntityId>,
    ) -> Result<RawComponent, RemapError> {
        let mut bytes = Cow::Borrowed(&self.bytes[..]);
        for field in self.fields.iter() {
            match field.primitive {
                Primitive::EntityId => {
                    let Some(slice) = bytes.to_mut().get_mut(field.offset..field.offset + 8) else {
                        return Err(RemapError::Eof);
                    };

                    let temp_entity =
                        EntityId::from_raw(u64::from_le_bytes(slice.try_into().unwrap()));
                    let Some(real_id) = get_entity(temp_entity) else {
                        return Err(RemapError::InvalidEntity);
                    };

                    let src = real_id.into_raw().to_le_bytes();
                    slice.copy_from_slice(&src);
                }
                Primitive::PlayerId => return Err(RemapError::Player),
                Primitive::Bytes => (),
            }
        }

        match bytes {
            Cow::Borrowed(_) => Ok(self),
            Cow::Owned(data) => Ok(Self {
                bytes: data.into(),
                fields: self.fields,
            }),
        }
    }
}

pub struct Iter<'a> {
    inner: std::collections::hash_map::Iter<'a, RecordReference, RawComponent>,
}

impl<'a> Iterator for Iter<'a> {
    type Item = (RecordReference, &'a RawComponent);

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

#[derive(Copy, Clone, Debug, Error)]
pub enum RemapError {
    #[error("unexpected eof")]
    Eof,
    #[error("contains invalid entity reference")]
    InvalidEntity,
    #[error("player reference not allowed")]
    Player,
}
