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
            pub(crate) const $id: RecordReference = RecordReference {
                module: ModuleId::CORE,
                record: RecordId($val),
            };
        )*
    };
}

// Must be kept in sync with `game_common/src/components/mod.rs`.
define_id! {
    TRANSFORM => 1,
    GLOBAL_TRANSFORM => 11,

    // Rendering
    MESH_INSTANCE => 2,
    DIRECTIONAL_LIGHT => 3,
    POINT_LIGHT => 4,
    SPOT_LIGHT => 5,
    PRIMARY_CAMERA => 8,

    // Physics
    RIGID_BODY => 6,
    COLLIDER => 7,

    // Game
    INVENTORY => 9,
    CHILDREN => 10,
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
            .map_err(|_| DecodeError::InvalidString)
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

    #[inline]
    pub const fn as_rgb(self) -> [f32; 3] {
        [self.0[0], self.0[1], self.0[2]]
    }

    #[inline]
    pub const fn as_rgba(self) -> [f32; 4] {
        self.0
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
    fn encode<W>(&self, writer: W)
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
            _ => Err(DecodeError::InvalidVariant {
                ident: stringify!(RigidBodyKind),
                value: tag.into(),
            }),
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
    Ball(Ball),
    Capsule(Capsule),
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
            Self::Ball(ball) => {
                2u8.encode(&mut writer);
                ball.encode(&mut writer);
            }
            Self::Capsule(capsule) => {
                3u8.encode(&mut writer);
                capsule.encode(&mut writer);
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
            2 => Ball::decode(reader).map(Self::Ball),
            3 => Capsule::decode(reader).map(Self::Capsule),
            _ => Err(DecodeError::InvalidVariant {
                ident: stringify!(ColliderShape),
                value: tag.into(),
            }),
        }
    }
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct Cuboid {
    pub hx: f32,
    pub hy: f32,
    pub hz: f32,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, Encode, Decode)]
pub struct PrimaryCamera;

impl Component for PrimaryCamera {
    const ID: RecordReference = PRIMARY_CAMERA;
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct Ball {
    pub radius: f32,
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct Capsule {
    pub axis: Axis,
    pub half_height: f32,
    pub radius: f32,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Axis {
    X,
    Y,
    Z,
}

impl Axis {
    /// Returns a unit vector representing the axis.
    #[inline]
    pub const fn to_vec3(self) -> Vec3 {
        match self {
            Self::X => Vec3::X,
            Self::Y => Vec3::Y,
            Self::Z => Vec3::Z,
        }
    }
}

impl Encode for Axis {
    fn encode<W>(&self, writer: W)
    where
        W: Writer,
    {
        let tag: u8 = match self {
            Self::X => 0,
            Self::Y => 1,
            Self::Z => 2,
        };

        tag.encode(writer);
    }
}

impl Decode for Axis {
    type Error = DecodeError;

    fn decode<R>(reader: R) -> Result<Self, Self::Error>
    where
        R: Reader,
    {
        let tag = u8::decode(reader)?;
        match tag {
            0 => Ok(Self::X),
            1 => Ok(Self::Y),
            2 => Ok(Self::Z),
            _ => Err(DecodeError::InvalidVariant {
                ident: "Axis",
                value: tag as u64,
            }),
        }
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Encode, Decode)]
pub struct GlobalTransform(pub Transform);

impl Component for GlobalTransform {
    const ID: RecordReference = GLOBAL_TRANSFORM;
}
