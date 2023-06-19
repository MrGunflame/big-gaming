use game_common::components::transform::Transform;
use game_render::mesh::Mesh;

use crate::GltfMaterial;

#[derive(Clone, Debug)]
pub struct GltfPrimitive {
    pub mesh: Mesh,
    pub material: GltfMaterial,
}

#[derive(Clone, Debug)]
pub struct GltfMesh {
    pub primitives: Vec<GltfPrimitive>,
}

#[derive(Clone, Debug)]
pub struct GltfNode {
    pub children: Vec<GltfNode>,
    pub mesh: Option<GltfMesh>,
    pub transform: Transform,
}
