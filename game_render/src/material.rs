use bevy_ecs::prelude::Bundle;

use crate::mesh::Mesh;

#[derive(Clone, Debug, Bundle)]
pub struct MaterialMeshBundle {
    pub mesh: Mesh,
}
