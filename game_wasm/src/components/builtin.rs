mod transform;

use alloc::string::String;
use alloc::vec::Vec;
use bytemuck::{Pod, Zeroable};
use glam::Vec3;

use crate::record::{ModuleId, RecordId, RecordReference};

use super::{Component, Decode, DecodeError, Encode};

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
    fn encode<B>(&self, mut buf: B)
    where
        B: bytes::BufMut,
    {
        buf.put_slice(self.path.as_bytes());
    }
}

impl Decode for MeshInstance {
    type Error = DecodeError;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: bytes::Buf,
    {
        let mut bytes = Vec::new();
        while buf.remaining() > 0 {
            bytes.push(buf.get_u8());
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
    fn encode<B>(&self, mut buf: B)
    where
        B: bytes::BufMut,
    {
        let tag = match self {
            Self::Fixed => 0,
            Self::Dynamic => 1,
            Self::Kinematic => 2,
        };

        buf.put_u8(tag);
    }
}

impl Decode for RigidBodyKind {
    type Error = DecodeError;

    fn decode<B>(buf: B) -> Result<Self, Self::Error>
    where
        B: bytes::Buf,
    {
        let tag = u8::decode(buf)?;
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
    fn encode<B>(&self, mut buf: B)
    where
        B: bytes::BufMut,
    {
        match self {
            Self::Cuboid(cuboid) => {
                buf.put_u8(1);
                cuboid.encode(buf)
            }
        };
    }
}

impl Decode for ColliderShape {
    type Error = DecodeError;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: bytes::Buf,
    {
        let tag = u8::decode(&mut buf)?;

        match tag {
            1 => Cuboid::decode(buf).map(Self::Cuboid),
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
