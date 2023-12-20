//! The world data format.

#[cfg(feature = "json")]
pub mod json;

use std::collections::HashMap;

use bytes::{Buf, BufMut};
use game_common::components::components::{Component, Components};
use game_common::components::transform::Transform;
use game_common::module::ModuleId;
use game_common::record::{RecordId, RecordReference};
use game_common::world::entity::EntityKind;
use game_common::world::terrain::TerrainMesh;
use game_common::world::CellId;
use glam::{Quat, Vec3};

#[derive(Clone, Debug)]
pub struct Cells {
    pub cells: HashMap<CellId, Vec<Entity>>,
}

impl Encode for Cells {
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        (self.cells.len() as u64).encode(&mut buf);
        for (id, entities) in &self.cells {
            id.encode(&mut buf);
            (entities.len() as u64).encode(&mut buf);
            for entity in entities {
                entity.encode(&mut buf);
            }
        }
    }
}

impl Decode for Cells {
    type Error = ();

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let len = u64::decode(&mut buf).unwrap();
        let mut cells = HashMap::new();
        for _ in 0..len {
            let id = CellId::decode(&mut buf).unwrap();
            let len = u64::decode(&mut buf).unwrap();
            let mut entities = Vec::new();
            for _ in 0..len {
                let entity = Entity::decode(&mut buf).unwrap();
                entities.push(entity);
            }

            cells.insert(id, entities);
        }

        Ok(Self { cells })
    }
}

impl Encode for CellId {
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        let (x, y, z) = self.as_parts();
        x.encode(&mut buf);
        y.encode(&mut buf);
        z.encode(&mut buf);
    }
}

impl Decode for CellId {
    type Error = ();

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let x = u32::decode(&mut buf).unwrap();
        let y = u32::decode(&mut buf).unwrap();
        let z = u32::decode(&mut buf).unwrap();

        Ok(CellId::from_parts(x, y, z))
    }
}

#[derive(Clone, Debug)]
pub struct Entity {
    pub id: RecordReference,
    pub kind: EntityKind,
    pub transform: Transform,
    pub components: Components,
    pub terrain: Option<TerrainMesh>,
}

impl Encode for Entity {
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        self.id.encode(&mut buf);
        self.kind.encode(&mut buf);

        self.transform.translation.x.encode(&mut buf);
        self.transform.translation.y.encode(&mut buf);
        self.transform.translation.z.encode(&mut buf);
        self.transform.rotation.x.encode(&mut buf);
        self.transform.rotation.y.encode(&mut buf);
        self.transform.rotation.z.encode(&mut buf);
        self.transform.rotation.w.encode(&mut buf);
        self.transform.scale.x.encode(&mut buf);
        self.transform.scale.y.encode(&mut buf);
        self.transform.scale.z.encode(&mut buf);

        (self.components.len() as u64).encode(&mut buf);
        for (id, comp) in self.components.iter() {
            id.encode(&mut buf);

            (comp.len() as u64).encode(&mut buf);
            buf.put_slice(comp.as_bytes());
        }
    }
}

impl Decode for Entity {
    type Error = ();

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let id = RecordReference::decode(&mut buf).unwrap();
        let kind = EntityKind::decode(&mut buf).unwrap();

        let tx = f32::decode(&mut buf).unwrap();
        let ty = f32::decode(&mut buf).unwrap();
        let tz = f32::decode(&mut buf).unwrap();
        let rx = f32::decode(&mut buf).unwrap();
        let ry = f32::decode(&mut buf).unwrap();
        let rz = f32::decode(&mut buf).unwrap();
        let rw = f32::decode(&mut buf).unwrap();
        let sx = f32::decode(&mut buf).unwrap();
        let sy = f32::decode(&mut buf).unwrap();
        let sz = f32::decode(&mut buf).unwrap();

        let transform = Transform {
            translation: Vec3::new(tx, ty, tz),
            rotation: Quat::from_xyzw(rx, ry, rz, rw),
            scale: Vec3::new(sx, sy, sz),
        };

        let len = u64::decode(&mut buf).unwrap();
        let mut components = Components::new();
        for _ in 0..len {
            let id = RecordReference::decode(&mut buf).unwrap();

            let len = u64::decode(&mut buf).unwrap();
            let mut bytes = vec![];
            for _ in 0..len {
                bytes.push(u8::decode(&mut buf).unwrap());
            }

            components.insert(id, Component::new(bytes));
        }

        Ok(Self {
            id,
            kind,
            transform,
            components,
            terrain: None,
        })
    }
}

pub trait Encode {
    fn encode<B>(&self, buf: B)
    where
        B: BufMut;
}

pub trait Decode: Sized {
    type Error;

    fn decode<B>(buf: B) -> Result<Self, Self::Error>
    where
        B: Buf;
}

impl Encode for RecordReference {
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        buf.put_slice(&self.module.into_bytes());
        buf.put_slice(&self.record.0.to_le_bytes());
    }
}

impl Decode for RecordReference {
    type Error = ();

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let mut module = [0; 16];
        buf.copy_to_slice(&mut module);
        let mut record = [0; 4];
        buf.copy_to_slice(&mut record);

        Ok(Self {
            module: ModuleId::from_bytes(module),
            record: RecordId(u32::from_le_bytes(record)),
        })
    }
}

impl Encode for EntityKind {
    fn encode<B>(&self, buf: B)
    where
        B: BufMut,
    {
        let b: u8 = match self {
            Self::Actor => 0,
            Self::Item => 1,
            Self::Object => 2,
            Self::Terrain => 3,
        };

        b.encode(buf)
    }
}

impl Decode for EntityKind {
    type Error = ();

    fn decode<B>(buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let b = u8::decode(buf).unwrap();

        match b {
            0 => Ok(Self::Actor),
            1 => Ok(Self::Item),
            2 => Ok(Self::Object),
            3 => Ok(Self::Terrain),
            _ => todo!(),
        }
    }
}

macro_rules! impl_primitive {
    ($($t:ty),*) => {
        $(
            impl Encode for $t {
                fn encode<B>(&self, mut buf: B)
                where
                    B: BufMut,
                {
                    buf.put_slice(&self.to_le_bytes());
                }
            }

            impl Decode for $t {
                type Error = ();

                fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
                where
                    B: Buf,
                {
                    let mut bytes = [0; std::mem::size_of::<Self>()];
                    buf.copy_to_slice(&mut bytes);
                    Ok(Self::from_le_bytes(bytes))
                }
            }
        )*
    };
}

impl_primitive! { u8, u16, u32, u64, i8, i16, i32, i64, f32, f64 }
