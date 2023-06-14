use bevy_ecs::prelude::Component;
use bytemuck::{Pod, Zeroable};
use game_asset::Asset;
use glam::{Vec2, Vec3};
use wgpu::{
    BufferAddress, PrimitiveTopology, VertexAttribute, VertexBufferLayout, VertexFormat,
    VertexStepMode,
};

// FIXME: Meshes will be duplicated quite a bit, so
// we don't want to have it attached to every entity.
#[derive(Clone, Debug, Component)]
pub struct Mesh {
    topology: PrimitiveTopology,
    indices: Option<Indices>,
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    tangents: Vec<[f32; 3]>,
    bitangents: Vec<[f32; 3]>,
    triangles_included: Vec<u32>,
}

impl Mesh {
    pub fn new() -> Self {
        Self {
            topology: PrimitiveTopology::TriangleList,
            indices: None,
            positions: vec![],
            normals: vec![],
            uvs: vec![],
            tangents: vec![],
            bitangents: vec![],
            triangles_included: vec![],
        }
    }

    pub fn set_indices(&mut self, indices: Indices) {
        self.indices = Some(indices);
    }

    pub fn set_positions(&mut self, positions: Vec<[f32; 3]>) {
        self.positions = positions;
    }

    pub fn positions(&self) -> &[[f32; 3]] {
        &self.positions
    }

    pub fn set_normals(&mut self, normals: Vec<[f32; 3]>) {
        self.normals = normals;
    }

    pub fn set_uvs(&mut self, uvs: Vec<[f32; 2]>) {
        self.uvs = uvs;
    }

    pub fn indicies(&self) -> Option<Indices> {
        self.indices.clone()
    }

    pub fn vertices(&self) -> Vec<Vertex> {
        let end = usize::max(
            usize::max(self.positions.len(), self.normals.len()),
            self.uvs.len(),
        );
        let mut index = 0;

        let mut vertices = Vec::with_capacity(end);

        while index < end {
            let position = self.positions.get(index).copied().unwrap_or_default();
            let normal = self.normals.get(index).copied().unwrap_or_default();
            let uv = self.uvs.get(index).copied().unwrap_or_default();
            let tangent = self.tangents.get(index).copied().unwrap_or_default();
            let bitangent = self.bitangents.get(index).copied().unwrap_or_default();

            vertices.push(Vertex {
                position,
                normal,
                uv,
                tangent,
                bitangent,
            });

            index += 1;
        }

        vertices
    }

    pub fn compute_tangents(&mut self) {
        self.tangents.clear();
        self.bitangents.clear();
        self.triangles_included.clear();

        let len = self.indices.as_ref().unwrap().len() as usize;

        self.tangents.resize(len, [0.0; 3]);
        self.bitangents.resize(len, [0.0; 3]);
        self.triangles_included.resize(len, 0);

        for c in self.indices.clone().unwrap().into_u32().chunks(3) {
            let pos0 = Vec3::from_array(self.positions[c[0] as usize]);
            let pos1 = Vec3::from_array(self.positions[c[1] as usize]);
            let pos2 = Vec3::from_array(self.positions[c[2] as usize]);

            let uv0 = Vec2::from_array(self.uvs[c[0] as usize]);
            let uv1 = Vec2::from_array(self.uvs[c[1] as usize]);
            let uv2 = Vec2::from_array(self.uvs[c[2] as usize]);

            let delta_pos1 = pos1 - pos0;
            let delta_pos2 = pos2 - pos0;

            let delta_uv1 = uv1 - uv0;
            let delta_uv2 = uv2 - uv0;

            let f = 1.0 / (delta_uv1.x * delta_uv2.y - delta_uv1.y * delta_uv2.x);
            let tangent = (delta_pos1 * delta_uv2.y - delta_pos2 * delta_uv1.y) * f;
            let bitangent = (delta_pos2 * delta_uv1.x - delta_pos1 * delta_uv2.x) * -f;

            self.tangents[c[0] as usize] =
                (Vec3::from_array(self.tangents[c[0] as usize]) + tangent).to_array();
            self.tangents[c[1] as usize] =
                (Vec3::from_array(self.tangents[c[1] as usize]) + tangent).to_array();
            self.tangents[c[2] as usize] =
                (Vec3::from_array(self.tangents[c[2] as usize]) + tangent).to_array();

            self.bitangents[c[0] as usize] =
                (Vec3::from_array(self.bitangents[c[0] as usize]) + bitangent).to_array();
            self.bitangents[c[1] as usize] =
                (Vec3::from_array(self.bitangents[c[1] as usize]) + bitangent).to_array();
            self.bitangents[c[2] as usize] =
                (Vec3::from_array(self.bitangents[c[2] as usize]) + bitangent).to_array();

            self.triangles_included[c[0] as usize] += 1;
            self.triangles_included[c[1] as usize] += 1;
            self.triangles_included[c[2] as usize] += 1;
        }

        // Average Tangents/Bitangents
        for (i, &n) in self.triangles_included.iter().enumerate() {
            let denom = 1.0 / n as f32;

            self.tangents[i] = (Vec3::from_array(self.tangents[i]) * denom).to_array();
            self.bitangents[i] = (Vec3::from_array(self.bitangents[i]) * denom).to_array();
        }
    }
}

#[derive(Clone, Debug)]
pub enum Indices {
    U16(Vec<u16>),
    U32(Vec<u32>),
}

impl Indices {
    pub fn len(&self) -> u32 {
        match self {
            Self::U16(buf) => buf.len() as u32,
            Self::U32(buf) => buf.len() as u32,
        }
    }

    pub fn as_u32(&self) -> &[u32] {
        match self {
            Self::U32(val) => val,
            _ => panic!("`Indicies` is not `U32`"),
        }
    }

    pub fn as_u16(&self) -> &[u16] {
        match self {
            Self::U16(val) => val,
            _ => panic!("`Indices` is not `U16`"),
        }
    }

    pub fn into_u32(self) -> Vec<u32> {
        match self {
            Self::U16(val) => val.into_iter().map(u32::from).collect(),
            Self::U32(val) => val,
        }
    }
}

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
pub struct Vertex {
    position: [f32; 3],
    normal: [f32; 3],
    uv: [f32; 2],
    tangent: [f32; 3],
    bitangent: [f32; 3],
}

impl Vertex {
    pub(crate) fn layout<'a>() -> VertexBufferLayout<'a> {
        VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as BufferAddress,
            step_mode: VertexStepMode::Vertex,
            attributes: &[
                VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: VertexFormat::Float32x3,
                },
                VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as BufferAddress,
                    shader_location: 1,
                    format: VertexFormat::Float32x3,
                },
                VertexAttribute {
                    offset: (std::mem::size_of::<[f32; 3]>() + std::mem::size_of::<[f32; 3]>())
                        as BufferAddress,
                    shader_location: 2,
                    format: VertexFormat::Float32x2,
                },
                VertexAttribute {
                    offset: (std::mem::size_of::<[f32; 3]>()
                        + std::mem::size_of::<[f32; 3]>()
                        + std::mem::size_of::<[f32; 2]>())
                        as BufferAddress,
                    shader_location: 3,
                    format: VertexFormat::Float32x3,
                },
                VertexAttribute {
                    offset: (std::mem::size_of::<[f32; 3]>()
                        + std::mem::size_of::<[f32; 3]>()
                        + std::mem::size_of::<[f32; 2]>()
                        + std::mem::size_of::<[f32; 3]>())
                        as BufferAddress,
                    shader_location: 4,
                    format: VertexFormat::Float32x3,
                },
            ],
        }
    }
}

impl Asset for Mesh {}

#[cfg(test)]
mod tests {
    use super::{Indices, Mesh};

    #[test]
    fn mesh_computed_tangents() {
        let mut mesh = Mesh::new();
        mesh.set_positions(vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 1.0, 0.0]]);
        mesh.set_uvs(vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0]]);
        mesh.set_indices(Indices::U32(vec![0, 1, 2]));
        mesh.set_normals(vec![[0.0, 0.0, 1.0], [0.0, 0.0, 1.0], [0.0, 0.0, 1.0]]);

        mesh.compute_tangents();

        assert_eq!(
            mesh.tangents,
            vec![[1.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 0.0, 0.0]]
        );
        assert_eq!(
            mesh.bitangents,
            vec![[0.0, -1.0, 0.0], [0.0, -1.0, 0.0], [0.0, -1.0, 0.0]]
        );
    }
}
