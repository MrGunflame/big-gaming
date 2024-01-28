use glam::{Vec2, Vec3};

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
            Vec3::new(s.min_x, s.min_y, s.max_z),
            Vec3::new(s.max_x, s.min_y, s.max_z),
            Vec3::new(s.max_x, s.max_y, s.max_z),
            Vec3::new(s.min_x, s.max_y, s.max_z),
            // Back
            Vec3::new(s.min_x, s.max_y, s.min_z),
            Vec3::new(s.max_x, s.max_y, s.min_z),
            Vec3::new(s.max_x, s.min_y, s.min_z),
            Vec3::new(s.min_x, s.min_y, s.min_z),
            // Right
            Vec3::new(s.max_x, s.min_y, s.min_z),
            Vec3::new(s.max_x, s.max_y, s.min_z),
            Vec3::new(s.max_x, s.max_y, s.max_z),
            Vec3::new(s.max_x, s.min_y, s.max_z),
            // Left
            Vec3::new(s.min_x, s.min_y, s.max_z),
            Vec3::new(s.min_x, s.max_y, s.max_z),
            Vec3::new(s.min_x, s.max_y, s.min_z),
            Vec3::new(s.min_x, s.min_y, s.min_z),
            // Top
            Vec3::new(s.max_x, s.max_y, s.min_z),
            Vec3::new(s.min_x, s.max_y, s.min_z),
            Vec3::new(s.min_x, s.max_y, s.max_z),
            Vec3::new(s.max_x, s.max_y, s.max_z),
            // Bottom
            Vec3::new(s.max_x, s.min_y, s.max_z),
            Vec3::new(s.min_x, s.min_y, s.max_z),
            Vec3::new(s.min_x, s.min_y, s.min_z),
            Vec3::new(s.max_x, s.min_y, s.min_z),
        ];

        let normals = [
            // Front
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(0.0, 0.0, 1.0),
            // Back
            Vec3::new(0.0, 0.0, -1.0),
            Vec3::new(0.0, 0.0, -1.0),
            Vec3::new(0.0, 0.0, -1.0),
            Vec3::new(0.0, 0.0, -1.0),
            // Right
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            // Left
            Vec3::new(-1.0, 0.0, 0.0),
            Vec3::new(-1.0, 0.0, 0.0),
            Vec3::new(-1.0, 0.0, 0.0),
            Vec3::new(-1.0, 0.0, 0.0),
            // Top
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            // Bottom
            Vec3::new(0.0, -1.0, 0.0),
            Vec3::new(0.0, -1.0, 0.0),
            Vec3::new(0.0, -1.0, 0.0),
            Vec3::new(0.0, -1.0, 0.0),
        ];

        let uvs = [
            // Front
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(0.0, 1.0),
            // Back
            Vec2::new(1.0, 0.0),
            Vec2::new(0.0, 0.0),
            Vec2::new(0.0, 1.0),
            Vec2::new(1.0, 1.0),
            // Right
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(0.0, 1.0),
            // Left
            Vec2::new(1.0, 0.0),
            Vec2::new(0.0, 0.0),
            Vec2::new(0.0, 1.0),
            Vec2::new(1.0, 1.0),
            // Top
            Vec2::new(1.0, 0.0),
            Vec2::new(0.0, 0.0),
            Vec2::new(0.0, 1.0),
            Vec2::new(1.0, 1.0),
            // Bottom
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(0.0, 1.0),
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
        mesh.set_positions(positions.to_vec());
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
            Vec3::new(max, 0.0, min),
            Vec3::new(min, 0.0, min),
            Vec3::new(min, 0.0, max),
            Vec3::new(max, 0.0, max),
        ];

        let normals = [
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];

        let uvs = [
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(0.0, 1.0),
            Vec2::new(1.0, 0.0),
        ];

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

impl Face {
    pub const fn inverse(self) -> Self {
        match self {
            Self::Front => Self::Back,
            Self::Back => Self::Front,
            Self::Left => Self::Right,
            Self::Right => Self::Left,
            Self::Top => Self::Bottom,
            Self::Bottom => Self::Top,
        }
    }
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
                    Vec3::new(min_x, min_y, 0.0),
                    Vec3::new(max_x, min_y, 0.0),
                    Vec3::new(max_x, max_y, 0.0),
                    Vec3::new(min_x, max_y, 0.0),
                ];
                normals = vec![
                    Vec3::new(0.0, 0.0, 1.0),
                    Vec3::new(0.0, 0.0, 1.0),
                    Vec3::new(0.0, 0.0, 1.0),
                    Vec3::new(0.0, 0.0, 1.0),
                ];
                uvs = vec![
                    Vec2::new(0.0, 0.0),
                    Vec2::new(1.0, 0.0),
                    Vec2::new(1.0, 1.0),
                    Vec2::new(0.0, 1.0),
                ];
            }
            Face::Back => {
                let min_x = -s.size.x / 2.0;
                let max_x = s.size.x / 2.0;
                let min_y = -s.size.y / 2.0;
                let max_y = s.size.y / 2.0;

                positions = vec![
                    Vec3::new(min_x, max_y, 0.0),
                    Vec3::new(max_x, max_y, 0.0),
                    Vec3::new(max_x, min_y, 0.0),
                    Vec3::new(min_x, min_y, 0.0),
                ];
                normals = vec![
                    Vec3::new(0.0, 0.0, -1.0),
                    Vec3::new(0.0, 0.0, -1.0),
                    Vec3::new(0.0, 0.0, -1.0),
                    Vec3::new(0.0, 0.0, -1.0),
                ];
                uvs = vec![
                    Vec2::new(1.0, 0.0),
                    Vec2::new(0.0, 0.0),
                    Vec2::new(0.0, 1.0),
                    Vec2::new(1.0, 1.0),
                ];
            }
            Face::Right => {
                let min_y = -s.size.y / 2.0;
                let max_y = s.size.y / 2.0;
                let min_z = -s.size.x / 2.0;
                let max_z = s.size.x / 2.0;

                positions = vec![
                    Vec3::new(0.0, min_y, min_z),
                    Vec3::new(0.0, max_y, min_z),
                    Vec3::new(0.0, max_y, max_z),
                    Vec3::new(0.0, min_y, max_z),
                ];
                normals = vec![
                    Vec3::new(1.0, 0.0, 0.0),
                    Vec3::new(1.0, 0.0, 0.0),
                    Vec3::new(1.0, 0.0, 0.0),
                    Vec3::new(1.0, 0.0, 0.0),
                ];
                uvs = vec![
                    Vec2::new(0.0, 0.0),
                    Vec2::new(1.0, 0.0),
                    Vec2::new(1.0, 1.0),
                    Vec2::new(0.0, 1.0),
                ];
            }
            Face::Left => {
                let min_y = -s.size.y / 2.0;
                let max_y = s.size.y / 2.0;
                let min_z = -s.size.x / 2.0;
                let max_z = s.size.x / 2.0;

                positions = vec![
                    Vec3::new(0.0, min_y, max_z),
                    Vec3::new(0.0, max_y, max_z),
                    Vec3::new(0.0, max_y, min_z),
                    Vec3::new(0.0, min_y, min_z),
                ];
                normals = vec![
                    Vec3::new(-1.0, 0.0, 0.0),
                    Vec3::new(-1.0, 0.0, 0.0),
                    Vec3::new(-1.0, 0.0, 0.0),
                    Vec3::new(-1.0, 0.0, 0.0),
                ];
                uvs = vec![
                    Vec2::new(1.0, 0.0),
                    Vec2::new(0.0, 0.0),
                    Vec2::new(0.0, 1.0),
                    Vec2::new(1.0, 1.0),
                ];
            }
            Face::Top => {
                let min_x = -s.size.x / 2.0;
                let max_x = s.size.x / 2.0;
                let min_z = -s.size.y / 2.0;
                let max_z = s.size.y / 2.0;

                positions = vec![
                    Vec3::new(max_x, 0.0, min_z),
                    Vec3::new(min_x, 0.0, min_z),
                    Vec3::new(min_x, 0.0, max_z),
                    Vec3::new(max_x, 0.0, max_z),
                ];
                normals = vec![
                    Vec3::new(0.0, 1.0, 0.0),
                    Vec3::new(0.0, 1.0, 0.0),
                    Vec3::new(0.0, 1.0, 0.0),
                    Vec3::new(0.0, 1.0, 0.0),
                ];
                uvs = vec![
                    Vec2::new(1.0, 0.0),
                    Vec2::new(0.0, 0.0),
                    Vec2::new(0.0, 1.0),
                    Vec2::new(1.0, 1.0),
                ];
            }
            Face::Bottom => {
                let min_x = -s.size.x / 2.0;
                let max_x = s.size.x / 2.0;
                let min_z = -s.size.y / 2.0;
                let max_z = s.size.y / 2.0;

                positions = vec![
                    Vec3::new(max_x, 0.0, max_z),
                    Vec3::new(min_x, 0.0, max_z),
                    Vec3::new(min_x, 0.0, min_z),
                    Vec3::new(max_x, 0.0, min_z),
                ];
                normals = vec![
                    Vec3::new(0.0, -1.0, 0.0),
                    Vec3::new(0.0, -1.0, 0.0),
                    Vec3::new(0.0, -1.0, 0.0),
                    Vec3::new(0.0, -1.0, 0.0),
                ];
                uvs = vec![
                    Vec2::new(0.0, 0.0),
                    Vec2::new(1.0, 0.0),
                    Vec2::new(1.0, 1.0),
                    Vec2::new(0.0, 1.0),
                ];
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
