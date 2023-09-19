use bytemuck::{Pod, Zeroable};
use game_common::components::transform::Transform;
use glam::{Mat3, Mat4, Vec4};
use slotmap::{DefaultKey, SlotMap};
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, Buffer, BufferUsages, Device, IndexFormat,
};

use crate::buffer::IndexBuffer;
use crate::forward::ForwardPipeline;
use crate::mesh::{Indices, Mesh};

pub fn update_mesh_bind_group(
    device: &Device,
    pipeline: &ForwardPipeline,
    mesh: &Mesh,
) -> (BindGroup, IndexBuffer) {
    // FIXME: Since meshes are user controlled, we might not catch invalid
    // meshes with a panic and simply ignore them.
    assert!(!mesh.positions().is_empty());
    assert!(!mesh.normals().is_empty());
    assert!(!mesh.tangents().is_empty());
    assert!(!mesh.uvs().is_empty());
    assert!(!mesh.indicies().as_ref().unwrap().is_empty());

    let indices = match mesh.indicies() {
        Some(Indices::U32(indices)) => {
            let buffer = device.create_buffer_init(&BufferInitDescriptor {
                label: Some("mesh_index_buffer"),
                contents: bytemuck::cast_slice(&indices),
                usage: BufferUsages::INDEX,
            });

            IndexBuffer {
                buffer,
                format: IndexFormat::Uint32,
                len: indices.len() as u32,
            }
        }
        Some(Indices::U16(indices)) => {
            let buffer = device.create_buffer_init(&BufferInitDescriptor {
                label: Some("mesh_index_buffer"),
                contents: bytemuck::cast_slice(&indices),
                usage: BufferUsages::INDEX,
            });

            IndexBuffer {
                buffer,
                format: IndexFormat::Uint16,
                len: indices.len() as u32,
            }
        }
        None => todo!(),
    };

    let positions = device.create_buffer_init(&BufferInitDescriptor {
        label: Some("mesh_vertex_positions"),
        contents: bytemuck::cast_slice(mesh.positions()),
        usage: BufferUsages::STORAGE,
    });

    let normals = device.create_buffer_init(&BufferInitDescriptor {
        label: Some("mesh_vertex_normals"),
        contents: bytemuck::cast_slice(mesh.normals()),
        usage: BufferUsages::STORAGE,
    });

    let tangents = device.create_buffer_init(&BufferInitDescriptor {
        label: Some("mesh_vertex_tangents"),
        contents: bytemuck::cast_slice(mesh.tangents()),
        usage: BufferUsages::STORAGE,
    });

    let uvs = device.create_buffer_init(&BufferInitDescriptor {
        label: Some("mesh_vertex_uvs"),
        contents: bytemuck::cast_slice(mesh.uvs()),
        usage: BufferUsages::STORAGE,
    });

    let mesh_bind_group = device.create_bind_group(&BindGroupDescriptor {
        label: Some("mesh_bind_group"),
        layout: &pipeline.mesh_bind_group_layout,
        entries: &[
            BindGroupEntry {
                binding: 0,
                resource: positions.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 1,
                resource: normals.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 2,
                resource: tangents.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 3,
                resource: uvs.as_entire_binding(),
            },
        ],
    });

    (mesh_bind_group, indices)
}

pub fn update_transform_buffer(transform: Transform, device: &Device) -> Buffer {
    device.create_buffer_init(&BufferInitDescriptor {
        label: Some("transform_buffer"),
        contents: bytemuck::bytes_of(&TransformUniform::from(transform)),
        usage: BufferUsages::UNIFORM,
    })
}

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
pub struct TransformUniform {
    transform: [[f32; 4]; 4],
    // rotation matrix for normals/tangents
    // Note that we can't use the transform matrix for non-uniform
    // scaling values.
    normal: [[f32; 4]; 3],
}

impl From<Transform> for TransformUniform {
    fn from(value: Transform) -> Self {
        let normal = Mat3::from_quat(value.rotation);
        let normal_x = Vec4::new(normal.x_axis.x, normal.x_axis.y, normal.x_axis.z, 0.0);
        let normal_y = Vec4::new(normal.y_axis.x, normal.y_axis.y, normal.y_axis.z, 0.0);
        let normal_z = Vec4::new(normal.z_axis.x, normal.z_axis.y, normal.z_axis.z, 0.0);

        Self {
            transform: Mat4::from_scale_rotation_translation(
                value.scale,
                value.rotation,
                value.translation,
            )
            .to_cols_array_2d(),
            normal: [
                normal_x.to_array(),
                normal_y.to_array(),
                normal_z.to_array(),
            ],
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct MeshId(DefaultKey);

#[derive(Clone, Debug)]
pub struct Meshes {
    meshes: SlotMap<DefaultKey, Mesh>,
}

impl Meshes {
    pub fn new() -> Self {
        Self {
            meshes: SlotMap::new(),
        }
    }

    pub fn insert(&mut self, mesh: Mesh) -> MeshId {
        let key = self.meshes.insert(mesh);
        MeshId(key)
    }

    pub fn remove(&mut self, id: MeshId) {
        self.meshes.remove(id.0);
    }

    pub fn get(&self, id: MeshId) -> Option<&Mesh> {
        self.meshes.get(id.0)
    }
}
