mod transform;

use alloc::string::String;
use alloc::vec::Vec;
use bytemuck::{Pod, Zeroable};
use glam::Vec3;

use super::Component;
use crate::encoding::{Decode, DecodeError, Encode, Primitive, Reader, Writer};
use crate::record::{ModuleId, RecordId, RecordReference};

pub use transform::Transform;

macro_rules! define_id {
    ($($id:ident => $val:expr),*,) => {
        $(
            const $id: RecordReference = RecordReference {
                module: ModuleId::CORE,
                record: RecordId($val),
            };
        )*
    };
}

// Must be kept in sync with `game_common/src/components/mod.rs`.
define_id! {
    TRANSFORM => 1,

    // Rendering
    MESH_INSTANCE => 2,
    DIRECTIONAL_LIGHT => 3,
    POINT_LIGHT => 4,
    SPOT_LIGHT => 5,

    // Physics
    RIGID_BODY => 6,
    COLLIDER => 7,
}

#[derive(Clone, Debug)]
pub struct MeshInstance {
    pub path: String,
}

impl Encode for MeshInstance {
    fn encode<W>(&self, mut writer: W)
    where
        W: Writer,
    {
        writer.write(Primitive::Bytes, self.path.as_bytes());
    }
}

impl Decode for MeshInstance {
    type Error = DecodeError;

    fn decode<R>(mut reader: R) -> Result<Self, Self::Error>
    where
        R: Reader,
    {
        let mut bytes = Vec::new();
        while reader.chunk().len() > 0 {
            bytes.push(reader.chunk()[0]);
            reader.advance(1);
        }

        String::from_utf8(bytes)
            .map_err(|_| DecodeError)
            .map(|path| Self { path })
    }
}

impl Component for MeshInstance {
    const ID: RecordReference = MESH_INSTANCE;
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct DirectionalLight {
    pub color: Color,
    pub illuminance: f32,
}

impl Component for DirectionalLight {
    const ID: RecordReference = DIRECTIONAL_LIGHT;
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct PointLight {
    pub color: Color,
    pub intensity: f32,
    pub radius: f32,
}

impl Component for PointLight {
    const ID: RecordReference = POINT_LIGHT;
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct SpotLight {
    pub color: Color,
    pub intensity: f32,
    pub radius: f32,
    /// Inner cutoff angle
    pub inner_cutoff: f32,
    pub outer_cutoff: f32,
}

impl Component for SpotLight {
    const ID: RecordReference = SPOT_LIGHT;
}

#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod, Encode, Decode)]
#[repr(transparent)]
pub struct Color(pub [f32; 4]);

impl Color {
    pub const WHITE: Self = Self([1.0, 1.0, 1.0, 1.0]);
    pub const BLACK: Self = Self([0.0, 0.0, 0.0, 1.0]);

    pub const RED: Self = Self([1.0, 0.0, 0.0, 1.0]);
    pub const GREEN: Self = Self([0.0, 1.0, 0.0, 1.0]);
    pub const BLUE: Self = Self([0.0, 0.0, 1.0, 1.0]);

    pub fn as_rgb(self) -> [f32; 3] {
        [self.0[0], self.0[1], self.0[2]]
    }

    #[inline]
    pub const fn from_rgb(rgb: [f32; 3]) -> Self {
        Self([rgb[0], rgb[1], rgb[2], 1.0])
    }

    #[inline]
    pub const fn from_rgba(rgba: [f32; 4]) -> Self {
        Self(rgba)
    }
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct RigidBody {
    pub kind: RigidBodyKind,
    pub linvel: Vec3,
    pub angvel: Vec3,
}

impl Component for RigidBody {
    const ID: RecordReference = RIGID_BODY;
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum RigidBodyKind {
    Fixed,
    Dynamic,
    Kinematic,
}

impl Encode for RigidBodyKind {
    fn encode<W>(&self, mut writer: W)
    where
        W: Writer,
    {
        let tag: u8 = match self {
            Self::Fixed => 0,
            Self::Dynamic => 1,
            Self::Kinematic => 2,
        };

        tag.encode(writer);
    }
}

impl Decode for RigidBodyKind {
    type Error = DecodeError;

    fn decode<R>(reader: R) -> Result<Self, Self::Error>
    where
        R: Reader,
    {
        let tag = u8::decode(reader)?;
        match tag {
            0 => Ok(Self::Fixed),
            1 => Ok(Self::Dynamic),
            2 => Ok(Self::Kinematic),
            _ => Err(DecodeError),
        }
    }
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct Collider {
    pub friction: f32,
    pub restitution: f32,
    pub shape: ColliderShape,
}

impl Component for Collider {
    const ID: RecordReference = COLLIDER;
}

#[derive(Clone, Debug)]
pub enum ColliderShape {
    Cuboid(Cuboid),
}

impl Encode for ColliderShape {
    fn encode<W>(&self, mut writer: W)
    where
        W: Writer,
    {
        match self {
            Self::Cuboid(cuboid) => {
                1u8.encode(&mut writer);
                cuboid.encode(&mut writer);
            }
        };
    }
}

impl Decode for ColliderShape {
    type Error = DecodeError;

    fn decode<R>(mut reader: R) -> Result<Self, Self::Error>
    where
        R: Reader,
    {
        let tag = u8::decode(&mut reader)?;

        match tag {
            1 => Cuboid::decode(reader).map(Self::Cuboid),
            _ => Err(DecodeError),
        }
    }
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct Cuboid {
    pub hx: f32,
    pub hy: f32,
    pub hz: f32,
}
