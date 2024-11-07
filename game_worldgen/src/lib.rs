use game_wasm::cell::CellId;
use game_wasm::components::builtin::Transform;
use game_wasm::world::RecordReference;
use glam::{Quat, Vec3};

#[derive(Clone, Debug)]
pub struct WorldgenState {
    entities: Vec<Entity>,
}

impl WorldgenState {
    pub fn new() -> Self {
        Self {
            entities: Vec::new(),
        }
    }

    pub fn insert(&mut self, entity: Entity) {
        self.entities.push(entity);
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        for entity in &self.entities {
            bytes.extend(entity.prefab.into_bytes());

            for b in entity.transform.translation.to_array() {
                bytes.extend(b.to_le_bytes());
            }

            debug_assert!(entity.transform.rotation.is_normalized());
            for b in entity.transform.rotation.to_array() {
                bytes.extend(b.to_le_bytes());
            }

            for b in entity.transform.scale.to_array() {
                bytes.extend(b.to_le_bytes());
            }
        }

        bytes
    }

    pub fn from_bytes(mut bytes: &[u8]) -> Result<Self, Error> {
        let mut entities = Vec::new();

        while !bytes.is_empty() {
            let Some(prefab) = bytes.get(0..20) else {
                return Err(Error {});
            };
            let Some(translation) = bytes.get(20..32) else {
                return Err(Error {});
            };
            let Some(rotation) = bytes.get(32..48) else {
                return Err(Error {});
            };
            let Some(scale) = bytes.get(48..60) else {
                return Err(Error {});
            };
            bytes = &bytes[60..];

            let prefab = RecordReference::from_bytes(prefab.try_into().unwrap());
            let translation = Vec3::from_array([
                f32::from_le_bytes(translation[0..4].try_into().unwrap()),
                f32::from_le_bytes(translation[4..8].try_into().unwrap()),
                f32::from_le_bytes(translation[8..12].try_into().unwrap()),
            ]);
            let rotation = Quat::from_array([
                f32::from_le_bytes(rotation[0..4].try_into().unwrap()),
                f32::from_le_bytes(rotation[4..8].try_into().unwrap()),
                f32::from_le_bytes(rotation[8..12].try_into().unwrap()),
                f32::from_le_bytes(rotation[12..16].try_into().unwrap()),
            ])
            .normalize();
            let scale = Vec3::from_array([
                f32::from_le_bytes(scale[0..4].try_into().unwrap()),
                f32::from_le_bytes(scale[4..8].try_into().unwrap()),
                f32::from_le_bytes(scale[8..12].try_into().unwrap()),
            ]);

            entities.push(Entity {
                prefab,
                transform: Transform {
                    translation,
                    rotation,
                    scale,
                },
            })
        }

        Ok(Self { entities })
    }

    pub fn load(&self, cell: CellId) -> EntitiesIter<'_> {
        EntitiesIter {
            iter: self.entities.iter(),
            cell,
        }
    }

    pub fn all(&self) -> impl Iterator<Item = &'_ Entity> + '_ {
        self.entities.iter()
    }

    pub fn extend(&mut self, other: Self) {
        self.entities.extend(other.entities);
    }
}

#[derive(Clone, Debug)]
pub struct Entity {
    pub prefab: RecordReference,
    pub transform: Transform,
}

#[derive(Clone, Debug)]
pub struct Error {}

pub struct EntitiesIter<'a> {
    iter: core::slice::Iter<'a, Entity>,
    cell: CellId,
}

impl<'a> Iterator for EntitiesIter<'a> {
    type Item = &'a Entity;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let entity = self.iter.next()?;
            if CellId::from(entity.transform.translation) == self.cell {
                return Some(entity);
            }
        }
    }
}
