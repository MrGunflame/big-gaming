use super::AsComponent;

#[derive(Copy, Clone, Debug)]
pub struct RigidBody {
    pub kind: RigidBodyKind,
}

#[derive(Copy, Clone, Debug)]
pub enum RigidBodyKind {
    Fixed,
    Dynamic,
    Kinematic,
}

impl AsComponent for RigidBody {
    const ID: crate::record::RecordReference = super::RIGID_BODY;

    fn from_bytes(buf: &[u8]) -> Self {
        let kind = match buf[0] {
            0 => RigidBodyKind::Fixed,
            1 => RigidBodyKind::Dynamic,
            2 => RigidBodyKind::Kinematic,
            _ => todo!(),
        };

        Self { kind }
    }

    fn to_bytes(&self) -> Vec<u8> {
        let kind = match self.kind {
            RigidBodyKind::Fixed => 0,
            RigidBodyKind::Dynamic => 1,
            RigidBodyKind::Kinematic => 2,
        };

        vec![kind]
    }
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
    const ID: crate::record::RecordReference = super::COLLIDER;

    fn from_bytes(buf: &[u8]) -> Self {
        let [friction, restitution, hx, hy, hz] = bytemuck::pod_read_unaligned::<[f32; 5]>(&buf);

        Self {
            friction,
            restitution,
            shape: ColliderShape::Cuboid(Cuboid { hx, hy, hz }),
        }
    }

    fn to_bytes(&self) -> Vec<u8> {
        let friction = bytemuck::bytes_of(&self.friction);
        let restitution = bytemuck::bytes_of(&self.restitution);
        let (hx, hy, hz) = match &self.shape {
            ColliderShape::Cuboid(cuboid) => (
                bytemuck::bytes_of(&cuboid.hx),
                bytemuck::bytes_of(&cuboid.hy),
                bytemuck::bytes_of(&cuboid.hz),
            ),
        };

        let mut bytes = Vec::new();
        bytes.extend(friction);
        bytes.extend(restitution);
        bytes.extend(hx);
        bytes.extend(hy);
        bytes.extend(hz);
        bytes
    }
}
