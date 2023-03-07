use core::panic;

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
        // let mut normals = Vec::new();

        let size_x = CELL_SIZE_UINT.x + 1;
        let size_z = CELL_SIZE_UINT.z + 1;

        for index in 0u32..size_x * size_z {
            let x = index % size_x;
            let z = index / size_z;

            let y = self.offsets.nodes[index as usize];

            vertices.push([x as f32, y as f32, z as f32]);
            // normals.push([0.0, 0.0, 1.0]);

            if x != size_x - 1 && z != size_z - 1 {
                // Up tri (index -> index + 10 -> index + 10 + 1)
                indices.extend([index, index + size_x, index + size_x + 1]);

                // Down tri (index -> index + 1 -> index + 10 + 1)
                indices.extend([index + size_x + 1, index + 1, index]);
            }
        }

        // let mut index = 0;
        // assert!(vertices.len() % 3 == 0);
        // while index < vertices.len() {
        //     let a = vertices[index];
        //     let b = vertices[index + 1];
        //     let c = vertices[index + 2];

        //     let (a, b, c) = (Vec3::from(a), Vec3::from(b), Vec3::from(c));
        //     let normal: [f32; 3] = (b - a).cross(c - a).normalize().into();

        //     normals.push([0.0, 0.0, 0.0]);

        //     normals.extend([normal, normal, normal]);

        //     index += 3;
        // }

        // for index in 0u32..vertices {
        //     let x = index % size_x;
        //     let z = index / size_z;

        //     if x == size_x - 1 || z == size_z - 1 {
        //         continue;
        //     }

        //     // Up tri
        //     let a = vertices[index as usize];
        //     let b = vertices[index as usize + size_x as usize];
        //     let c = vertices[index as usize + size_x as usize + 1];

        //     let (a, b, c) = (Vec3::from(a), Vec3::from(b), Vec3::from(c));
        //     let normal: [f32; 3] = (b - a).cross(c - a).normalize().into();

        //     normals.push(normal);

        //     // Down tri
        //     // let a = vertices[index as usize + size_x as usize + 1];
        //     // let b = vertices[index as usize + 1];
        //     // let c = vertices[index as usize];
        //     // dbg!((a, b, c));

        //     // let (a, b, c) = (Vec3::from(a), Vec3::from(b), Vec3::from(c));
        //     // let normal: [f32; 3] = (b - a).cross(c - a).normalize().into();

        //     // normals.push(normal);
        // }

        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
        mesh.set_indices(Some(Indices::U32(indices)));
        // mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);

        mesh.duplicate_vertices();
        mesh.compute_flat_normals();

        // dbg!(mesh.attribute(Mesh::ATTRIBUTE_NORMAL));
        // panic!();

        mesh
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Heightmap {
    pub nodes: Vec<f32>,
}
