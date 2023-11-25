use game_common::components::transform::Transform;
use game_core::hierarchy::Hierarchy;
use game_render::color::Color;
use game_render::pbr::mesh::MeshId;
use game_render::pbr::PbrMaterial;
use glam::Vec3;

#[derive(Clone, Debug)]
pub struct Scene {
    pub nodes: Hierarchy<Node>,
}

/// A node in a scene graph.
#[derive(Clone, Debug)]
pub struct Node {
    pub transform: Transform,
    pub body: NodeBody,
}

#[derive(Clone, Debug)]
pub enum NodeBody {
    MeshInstance(MeshInstance),
    DirectionalLight(DirectionalLight),
    PointLight(PointLight),
    SpotLight(SpotLight),
    Collider(Collider),
}

#[derive(Copy, Clone, Debug)]
pub struct MeshInstance {
    pub mesh: MeshId,
    // TODO: Move out into `MaterialId`.
    pub material: PbrMaterial,
}

#[derive(Copy, Clone, Debug)]
pub struct DirectionalLight {
    pub color: Color,
    pub illuminance: f32,
}

#[derive(Copy, Clone, Debug)]
pub struct PointLight {
    pub color: Color,
    pub intensity: f32,
    pub radius: f32,
}

#[derive(Copy, Clone, Debug)]
pub struct SpotLight {
    pub color: Color,
    pub intensity: f32,
    pub radius: f32,
    pub inner_cutoff: f32,
    pub outer_cutoff: f32,
}

#[derive(Clone, Debug)]
pub struct Collider {
    // TOOD: More fine-grained control of center of mass, etc..
    pub mass: f32,
    pub friction: f32,
    pub restitution: f32,
    pub shape: ColliderShape,
}

#[derive(Clone, Debug)]
pub enum ColliderShape {
    Cuboid(Cuboid),
    TriMesh(TriMesh),
}

#[derive(Copy, Clone, Debug)]
pub struct Cuboid {
    pub hx: f32,
    pub hy: f32,
    pub hz: f32,
}

#[derive(Clone, Debug)]
pub struct TriMesh {
    pub vertices: Vec<Vec3>,
    pub indices: Vec<u32>,
}
