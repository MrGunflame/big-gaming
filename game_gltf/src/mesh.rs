use game_common::components::Transform;
use game_render::mesh::{Indices, Mesh};

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

pub(crate) fn validate_mesh(mesh: &Mesh) {
    // Indices must be in bounds.
    if let Some(indices) = mesh.indicies() {
        match indices {
            Indices::U16(indices) => {
                for index in indices {
                    assert!(mesh.positions().len() > index as usize);
                }
            }
            Indices::U32(indices) => {
                for index in indices {
                    assert!(mesh.positions().len() > index as usize);
                }
            }
        }
    }
}
