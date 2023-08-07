use bevy_ecs::prelude::Entity;
use bevy_ecs::query::{Added, Changed, Or, With};
use bevy_ecs::system::{Query, Res, ResMut};
use bytemuck::{Pod, Zeroable};
use game_asset::{Assets, Handle};
use game_common::components::transform::{GlobalTransform, Transform};
use glam::{Mat3, Mat4, Vec4};
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{BindGroupDescriptor, BindGroupEntry, BufferUsages, IndexFormat};

use crate::buffer::IndexBuffer;
use crate::forward::ForwardPipeline;
use crate::mesh::{Indices, Mesh};
use crate::render_pass::RenderNodes;
use crate::RenderDevice;

pub fn update_mesh_bind_group(
    device: Res<RenderDevice>,
    nodes: Query<(Entity, &Handle<Mesh>), Or<(Changed<Handle<Mesh>>, Added<Handle<Mesh>>)>>,
    meshes: Res<Assets<Mesh>>,
    pipeline: Res<ForwardPipeline>,
    mut render_nodes: ResMut<RenderNodes>,
) {
    for (entity, handle) in &nodes {
        let Some(mesh) = meshes.get(handle.id()) else {
            continue;
        };

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

        let node = render_nodes.entities.entry(entity).or_default();
        node.indices = Some(indices);
        node.mesh_bind_group = Some(mesh_bind_group);
    }
}

pub fn update_mesh_transform(
    device: Res<RenderDevice>,
    nodes: Query<
        (Entity, &GlobalTransform),
        (
            Or<(Added<GlobalTransform>, Changed<GlobalTransform>)>,
            With<Handle<Mesh>>,
        ),
    >,
    mut render_nodes: ResMut<RenderNodes>,
) {
    for (entity, transform) in &nodes {
        let buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("model_transform_buffer"),
            contents: bytemuck::cast_slice(&[TransformUniform::from(transform.0)]),
            usage: BufferUsages::UNIFORM,
        });

        let node = render_nodes.entities.entry(entity).or_default();
        node.transform = Some(buffer);
    }
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
