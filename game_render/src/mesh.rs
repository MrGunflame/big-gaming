use bevy_ecs::prelude::Component;
use bytemuck::{Pod, Zeroable};
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
}

impl Mesh {
    pub fn new() -> Self {
        Self {
            topology: PrimitiveTopology::TriangleList,
            indices: None,
            positions: vec![],
            normals: vec![],
            uvs: vec![],
        }
    }

    pub fn set_indices(&mut self, indices: Indices) {
        self.indices = Some(indices);
    }

    pub fn set_positions(&mut self, positions: Vec<[f32; 3]>) {
        self.positions = positions;
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
        self.positions
            .iter()
            .zip(self.normals.iter())
            .zip(self.uvs.iter())
            .map(|((pos, norm), uv)| Vertex {
                position: *pos,
                normal: *norm,
                uv: *uv,
            })
            .collect()
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
}

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
pub struct Vertex {
    position: [f32; 3],
    normal: [f32; 3],
    uv: [f32; 2],
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
            ],
        }
    }
}
