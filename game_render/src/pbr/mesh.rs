use bevy_ecs::prelude::Entity;
use bevy_ecs::query::{Added, Changed, Or, With};
use bevy_ecs::system::{Query, Res, ResMut};
use game_asset::{Assets, Handle};
use game_common::components::transform::GlobalTransform;
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

        let vertices = mesh.vertices();

        let vertices = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("mesh_vertex_buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: BufferUsages::VERTEX,
        });

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
        node.vertices = Some(vertices);
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
            contents: bytemuck::cast_slice(&[transform.0.compute_matrix()]),
            usage: BufferUsages::UNIFORM,
        });

        let node = render_nodes.entities.entry(entity).or_default();
        node.transform = Some(buffer);
    }
}
