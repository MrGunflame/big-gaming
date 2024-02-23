use gen::StaticGenerator;
use serde::{Deserialize, Serialize};

pub mod gen;

pub fn from_slice(slice: &[u8]) -> Result<StaticGenerator, Box<dyn std::error::Error>> {
    let entities: Vec<Entity> = serde_json::from_slice(slice)?;
    Ok(StaticGenerator { entities })
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Entity {
    components: Vec<Component>,
    transform: Transform,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
enum Component {
    Collider(Collider),
    MeshInstance(String),
    RigidBody(RigidBody),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Transform {
    translation: [f32; 3],
    rotation: [f32; 4],
    scale: [f32; 3],
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
struct Collider {
    friction: f32,
    restitution: f32,
    shape: ColliderShape,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
enum ColliderShape {
    Cuboid(Cuboid),
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
struct Cuboid {
    x: f32,
    y: f32,
    z: f32,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
struct RigidBody {
    linvel: [f32; 3],
    angvel: [f32; 3],
    kind: RigidBodyKind,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
enum RigidBodyKind {
    Fixed,
    Dynamic,
}
