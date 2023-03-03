use bevy_rapier3d::prelude::Collider;
use bevy_render::mesh::Indices;
use bevy_render::prelude::Mesh;
use bevy_render::render_resource::PrimitiveTopology;
use glam::Vec3;

use super::{CellId, CELL_SIZE_UINT};

#[derive(Clone, Debug, PartialEq)]
pub struct TerrainMesh {
    pub cell: CellId,
    offsets: Heightmap,
}

impl TerrainMesh {
    pub fn new(cell: CellId, offsets: Heightmap) -> Self {
        Self { cell, offsets }
    }

    pub fn collider(&self) -> Collider {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        let size_x = CELL_SIZE_UINT.x + 1;
        let size_z = CELL_SIZE_UINT.z + 1;

        for index in 0u32..size_x * size_z {
            let x = index % size_x;
            let z = index / size_z;

            let y = self.offsets.nodes[index as usize];

            vertices.push(Vec3::new(x as f32, y as f32, z as f32));

            if x != size_x - 1 && z != size_z - 1 {
                // Up tri (index -> index + 10 -> index + 10 + 1)
                indices.push([index, index + size_x, index + size_x + 1]);

                // Down tri (index -> index + 1 -> index + 10 + 1)
                indices.push([index + size_x + 1, index + 1, index]);
            }
        }

        Collider::trimesh(vertices, indices)
    }

    pub fn mesh(&self) -> Mesh {
        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);

        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        let size_x = CELL_SIZE_UINT.x + 1;
        let size_z = CELL_SIZE_UINT.z + 1;

        for index in 0u32..size_x * size_z {
            let x = index % size_x;
            let z = index / size_z;

            let y = self.offsets.nodes[index as usize];

            vertices.push([x as f32, y as f32, z as f32]);

            if x != size_x - 1 && z != size_z - 1 {
                // Up tri (index -> index + 10 -> index + 10 + 1)
                indices.extend([index, index + size_x, index + size_x + 1]);

                // Down tri (index -> index + 1 -> index + 10 + 1)
                indices.extend([index + size_x + 1, index + 1, index]);
            }
        }

        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
        mesh.set_indices(Some(Indices::U32(indices)));

        mesh
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Heightmap {
    pub nodes: Vec<f32>,
}
