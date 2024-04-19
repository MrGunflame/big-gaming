use bytemuck::{Pod, Zeroable};
use game_common::components::Transform;
use glam::{Mat3, Mat4, Vec4};
use slotmap::{DefaultKey, SlotMap};
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, Buffer, BufferUsages, Device, IndexFormat,
};

use crate::buffer::IndexBuffer;
use crate::forward::ForwardPipeline;
use crate::mesh::{Indices, Mesh};

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
    normal: [[f32; 4]; 4],
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
                [0.0, 0.0, 0.0, 0.0],
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
        tracing::trace!("inserting mesh {:?}", key);
        MeshId(key)
    }

    pub fn remove(&mut self, id: MeshId) {
        if self.meshes.remove(id.0).is_some() {
            tracing::trace!("removing mesh {:?}", id);
        }
    }

    pub fn get(&self, id: MeshId) -> Option<&Mesh> {
        self.meshes.get(id.0)
    }

    pub fn contains_key(&self, key: MeshId) -> bool {
        self.meshes.contains_key(key.0)
    }
}

impl Default for Meshes {
    fn default() -> Self {
        Self::new()
    }
}
