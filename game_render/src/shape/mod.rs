use crate::mesh::{Indices, Mesh};

#[derive(Copy, Clone, Debug)]
pub struct Box {
    pub min_x: f32,
    pub max_x: f32,
    pub min_y: f32,
    pub max_y: f32,
    pub min_z: f32,
    pub max_z: f32,
}

impl From<Box> for Mesh {
    fn from(s: Box) -> Self {
        let positions = [
            // Front
            [s.min_x, s.min_y, s.max_z],
            [s.max_x, s.min_y, s.max_z],
            [s.max_x, s.max_y, s.max_z],
            [s.min_x, s.max_y, s.max_z],
            // Back
            [s.min_x, s.max_y, s.min_z],
            [s.max_x, s.max_y, s.min_z],
            [s.max_x, s.min_y, s.min_z],
            [s.min_x, s.min_y, s.min_z],
            // Right
            [s.max_x, s.min_y, s.min_z],
            [s.max_x, s.max_y, s.min_z],
            [s.max_x, s.max_y, s.max_z],
            [s.max_x, s.min_y, s.max_z],
            // Left
            [s.min_x, s.min_y, s.max_z],
            [s.min_x, s.max_y, s.max_z],
            [s.min_x, s.max_y, s.min_z],
            [s.min_x, s.min_y, s.min_z],
            // Top
            [s.max_x, s.max_y, s.min_z],
            [s.min_x, s.max_y, s.min_z],
            [s.min_x, s.max_y, s.max_z],
            [s.max_x, s.max_y, s.max_z],
            // Bottom
            [s.max_x, s.min_y, s.max_z],
            [s.min_x, s.min_y, s.max_z],
            [s.min_x, s.min_y, s.min_z],
            [s.max_x, s.min_y, s.min_z],
        ];

        let normals = [
            // Front
            [0.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
            // Back
            [0.0, 0.0, -1.0],
            [0.0, 0.0, -1.0],
            [0.0, 0.0, -1.0],
            [0.0, 0.0, -1.0],
            // Right
            [1.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            // Left
            [-1.0, 0.0, 0.0],
            [-1.0, 0.0, 0.0],
            [-1.0, 0.0, 0.0],
            [-1.0, 0.0, 0.0],
            // Top
            [0.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
            // Bottom
            [0.0, -1.0, 0.0],
            [0.0, -1.0, 0.0],
            [0.0, -1.0, 0.0],
            [0.0, -1.0, 0.0],
        ];

        let uvs = [
            // Front
            [0.0, 0.0],
            [1.0, 0.0],
            [1.0, 1.0],
            [0.0, 1.0],
            // Back
            [1.0, 0.0],
            [0.0, 0.0],
            [0.0, 1.0],
            [1.0, 1.0],
            // Right
            [0.0, 0.0],
            [1.0, 0.0],
            [1.0, 1.0],
            [0.0, 1.0],
            // Left
            [1.0, 0.0],
            [0.0, 0.0],
            [0.0, 1.0],
            [1.0, 1.0],
            // Top
            [1.0, 0.0],
            [0.0, 0.0],
            [0.0, 1.0],
            [1.0, 1.0],
            // Bottom
            [0.0, 0.0],
            [1.0, 0.0],
            [1.0, 1.0],
            [0.0, 1.0],
        ];

        let indicies = vec![
            0, 1, 2, 2, 3, 0, // Front
            4, 5, 6, 6, 7, 4, // Back
            8, 9, 10, 10, 11, 8, // Right
            12, 13, 14, 14, 15, 12, // Left
            16, 17, 18, 18, 19, 16, // Top
            20, 21, 22, 22, 23, 20, // Bottom
        ];

        let mut mesh = Mesh::new();
        mesh.set_positions(positions.iter().copied().collect());
        mesh.set_indices(Indices::U32(indicies));
        mesh.set_normals(normals.to_vec());
        mesh.set_uvs(uvs.to_vec());
        mesh
    }
}

pub struct Plane {
    pub size: f32,
}

impl From<Plane> for Mesh {
    fn from(s: Plane) -> Self {
        let min = -s.size / 2.0;
        let max = s.size / 2.0;

        let positions = [
            [max, 0.0, min],
            [min, 0.0, min],
            [min, 0.0, max],
            [max, 0.0, max],
        ];

        let normals = [
            [0.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
        ];

        let uvs = [[0.0, 0.0], [1.0, 1.0], [0.0, 1.0], [1.0, 0.0]];

        let indicies = vec![0, 1, 2, 2, 3, 0];

        let mut mesh = Mesh::new();
        mesh.set_positions(positions.to_vec());
        mesh.set_indices(Indices::U32(indicies));
        mesh.set_normals(normals.to_vec());
        mesh.set_uvs(uvs.to_vec());
        mesh
    }
}
