use alloc::borrow::ToOwned;
use alloc::string::String;
use alloc::vec::Vec;
use bytemuck::{Pod, Zeroable};
use glam::{Quat, Vec3};

use crate::record::{ModuleId, RecordId, RecordReference};

use super::AsComponent;

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

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Transform {
    pub const IDENTITY: Self = Self {
        translation: Vec3::ZERO,
        rotation: Quat::IDENTITY,
        scale: Vec3::ONE,
    };
}

impl Default for Transform {
    fn default() -> Self {
        Self::IDENTITY
    }
}

#[derive(Clone, Debug)]
pub struct MeshInstance {
    pub path: String,
}

impl AsComponent for MeshInstance {
    const ID: RecordReference = MESH_INSTANCE;

    fn from_bytes(buf: &[u8]) -> Self {
        let s = core::str::from_utf8(buf).unwrap();
        Self { path: s.to_owned() }
    }

    fn to_bytes(&self) -> Vec<u8> {
        self.path.as_bytes().to_owned()
    }
}

impl AsComponent for Transform {
    const ID: RecordReference = TRANSFORM;

    fn from_bytes(buf: &[u8]) -> Self {
        let translation: [f32; 3] = bytemuck::pod_read_unaligned(&buf[0..4 * 3]);
        let rotation: [f32; 4] = bytemuck::pod_read_unaligned(&buf[4 * 3..4 * 3 + 4 * 4]);
        let scale: [f32; 3] = bytemuck::pod_read_unaligned(&buf[4 * 7..4 * 7 + 3 * 4]);

        Self {
            translation: Vec3::from_array(translation),
            rotation: Quat::from_array(rotation),
            scale: Vec3::from_array(scale),
        }
    }

    fn to_bytes(&self) -> Vec<u8> {
        let translation = self.translation.to_array();
        let rotation = self.rotation.to_array();
        let scale = self.scale.to_array();

        let mut bytes = Vec::new();
        bytes.extend(bytemuck::bytes_of(&translation));
        bytes.extend(bytemuck::bytes_of(&rotation));
        bytes.extend(bytemuck::bytes_of(&scale));
        bytes
    }
}

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
pub struct PointLight {
    pub color: Color,
    pub intensity: f32,
    pub radius: f32,
}

impl AsComponent for PointLight {
    const ID: RecordReference = POINT_LIGHT;

    fn from_bytes(buf: &[u8]) -> Self {
        bytemuck::pod_read_unaligned(buf)
    }

    fn to_bytes(&self) -> Vec<u8> {
        bytemuck::bytes_of(self).to_vec()
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
#[repr(transparent)]
pub struct Color(pub [f32; 4]);

impl Color {
    pub const WHITE: Self = Self([1.0, 1.0, 1.0, 1.0]);
}

#[derive(Clone, Debug)]
pub struct RigidBody {
    pub kind: RigidBodyKind,
    pub linvel: Vec3,
    pub angvel: Vec3,
}

impl AsComponent for RigidBody {
    const ID: RecordReference = RIGID_BODY;

    fn from_bytes(buf: &[u8]) -> Self {
        let kind = match buf[0] {
            0 => RigidBodyKind::Fixed,
            1 => RigidBodyKind::Dynamic,
            2 => RigidBodyKind::Kinematic,
            _ => todo!(),
        };

        let linvel: [f32; 3] = bytemuck::pod_read_unaligned(&buf[1..1 + 4 * 3]);
        let angvel: [f32; 3] = bytemuck::pod_read_unaligned(&buf[1 + 4 * 3..]);

        Self {
            kind,
            linvel: Vec3::from_array(linvel),
            angvel: Vec3::from_array(angvel),
        }
    }

    fn to_bytes(&self) -> Vec<u8> {
        let kind = match self.kind {
            RigidBodyKind::Fixed => 0,
            RigidBodyKind::Dynamic => 1,
            RigidBodyKind::Kinematic => 2,
        };

        let mut bytes = alloc::vec![kind];

        bytes.extend(bytemuck::bytes_of(&self.linvel));
        bytes.extend(bytemuck::bytes_of(&self.angvel));
        bytes
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum RigidBodyKind {
    Fixed,
    Dynamic,
    Kinematic,
}

#[derive(Clone, Debug)]
pub struct Collider {
    pub friction: f32,
    pub restitution: f32,
    pub shape: ColliderShape,
}

#[derive(Clone, Debug)]
pub enum ColliderShape {
    Cuboid(Cuboid),
}

#[derive(Copy, Clone, Debug)]
pub struct Cuboid {
    pub hx: f32,
    pub hy: f32,
    pub hz: f32,
}

impl AsComponent for Collider {
    const ID: RecordReference = COLLIDER;

    fn from_bytes(buf: &[u8]) -> Self {
        let [friction, restitution, hx, hy, hz] = bytemuck::pod_read_unaligned::<[f32; 5]>(buf);
        Self {
            friction,
            restitution,
            shape: ColliderShape::Cuboid(Cuboid { hx, hy, hz }),
        }
    }

    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend(bytemuck::bytes_of(&self.friction));
        bytes.extend(bytemuck::bytes_of(&self.restitution));

        match self.shape {
            ColliderShape::Cuboid(cuboid) => {
                bytes.extend(bytemuck::bytes_of(&cuboid.hx));
                bytes.extend(bytemuck::bytes_of(&cuboid.hy));
                bytes.extend(bytemuck::bytes_of(&cuboid.hz));
            }
        }

        bytes
    }
}
