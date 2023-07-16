use glam::Vec2;

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
        mesh.compute_tangents();
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
        mesh.compute_tangents();
        mesh
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Face {
    Front,
    Back,
    Left,
    Right,
    Top,
    Bottom,
}

/// A quad placed on a face.
#[derive(Copy, Clone, Debug)]
pub struct Quad {
    pub size: Vec2,
    pub face: Face,
}

impl From<Quad> for Mesh {
    fn from(s: Quad) -> Self {
        let positions;
        let normals;
        let uvs;

        match s.face {
            Face::Front => {
                let min_x = -s.size.x / 2.0;
                let max_x = s.size.x / 2.0;
                let min_y = -s.size.y / 2.0;
                let max_y = s.size.y / 2.0;

                positions = vec![
                    [min_x, min_y, 0.0],
                    [max_x, min_y, 0.0],
                    [max_x, max_y, 0.0],
                    [min_x, max_y, 0.0],
                ];
                normals = vec![
                    [0.0, 0.0, 1.0],
                    [0.0, 0.0, 1.0],
                    [0.0, 0.0, 1.0],
                    [0.0, 0.0, 1.0],
                ];
                uvs = vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
            }
            Face::Back => {
                let min_x = -s.size.x / 2.0;
                let max_x = s.size.x / 2.0;
                let min_y = -s.size.y / 2.0;
                let max_y = s.size.y / 2.0;

                positions = vec![
                    [min_x, max_y, 0.0],
                    [max_x, max_y, 0.0],
                    [max_x, min_y, 0.0],
                    [min_x, min_y, 0.0],
                ];
                normals = vec![
                    [0.0, 0.0, -1.0],
                    [0.0, 0.0, -1.0],
                    [0.0, 0.0, -1.0],
                    [0.0, 0.0, -1.0],
                ];
                uvs = vec![[1.0, 0.0], [0.0, 0.0], [0.0, 1.0], [1.0, 1.0]];
            }
            Face::Right => {
                let min_y = -s.size.y / 2.0;
                let max_y = s.size.y / 2.0;
                let min_z = -s.size.x / 2.0;
                let max_z = s.size.x / 2.0;

                positions = vec![
                    [0.0, min_y, min_z],
                    [0.0, max_y, min_z],
                    [0.0, max_y, max_z],
                    [0.0, min_y, max_z],
                ];
                normals = vec![
                    [1.0, 0.0, 0.0],
                    [1.0, 0.0, 0.0],
                    [1.0, 0.0, 0.0],
                    [1.0, 0.0, 0.0],
                ];
                uvs = vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
            }
            Face::Left => {
                let min_y = -s.size.y / 2.0;
                let max_y = s.size.y / 2.0;
                let min_z = -s.size.x / 2.0;
                let max_z = s.size.x / 2.0;

                positions = vec![
                    [0.0, min_y, max_z],
                    [0.0, max_y, max_z],
                    [0.0, max_y, min_z],
                    [0.0, min_y, min_z],
                ];
                normals = vec![
                    [-1.0, 0.0, 0.0],
                    [-1.0, 0.0, 0.0],
                    [-1.0, 0.0, 0.0],
                    [-1.0, 0.0, 0.0],
                ];
                uvs = vec![[1.0, 0.0], [0.0, 0.0], [0.0, 1.0], [1.0, 1.0]];
            }
            Face::Top => {
                let min_x = -s.size.x / 2.0;
                let max_x = s.size.x / 2.0;
                let min_z = -s.size.y / 2.0;
                let max_z = s.size.y / 2.0;

                positions = vec![
                    [max_x, 0.0, min_z],
                    [min_x, 0.0, min_z],
                    [min_x, 0.0, max_z],
                    [max_x, 0.0, max_z],
                ];
                normals = vec![
                    [0.0, 1.0, 0.0],
                    [0.0, 1.0, 0.0],
                    [0.0, 1.0, 0.0],
                    [0.0, 1.0, 0.0],
                ];
                uvs = vec![[1.0, 0.0], [0.0, 0.0], [0.0, 1.0], [1.0, 1.0]];
            }
            Face::Bottom => {
                let min_x = -s.size.x / 2.0;
                let max_x = s.size.x / 2.0;
                let min_z = -s.size.y / 2.0;
                let max_z = s.size.y / 2.0;

                positions = vec![
                    [max_x, 0.0, max_z],
                    [min_x, 0.0, max_z],
                    [min_x, 0.0, min_z],
                    [max_x, 0.0, min_z],
                ];
                normals = vec![
                    [0.0, -1.0, 0.0],
                    [0.0, -1.0, 0.0],
                    [0.0, -1.0, 0.0],
                    [0.0, -1.0, 0.0],
                ];
                uvs = vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
            }
        }

        let indices = Indices::U16(vec![0, 1, 2, 2, 3, 0]);

        let mut mesh = Mesh::new();
        mesh.set_positions(positions);
        mesh.set_normals(normals);
        mesh.set_indices(indices);
        mesh.set_uvs(uvs);
        mesh.compute_tangents();
        mesh
    }
}
