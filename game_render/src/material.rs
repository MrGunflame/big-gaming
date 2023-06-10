use std::sync::Arc;

use bevy_ecs::prelude::{Component, Res};
use bevy_ecs::query::{Added, Changed, Or};
use bevy_ecs::system::Query;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{
    Buffer, BufferUsages, Extent3d, ImageCopyTexture, ImageDataLayout, Origin3d, Texture,
    TextureAspect, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
};

use crate::mesh::Mesh;
use crate::{RenderDevice, RenderQueue};

#[derive(Debug, Default, Component)]
pub struct ComputedMesh {
    pub(crate) vertices: Option<Arc<Buffer>>,
    pub(crate) indicies: Option<Arc<Buffer>>,
    pub(crate) num_vertices: u32,
}

#[derive(Debug, Default, Component)]
pub struct ComputedMaterial {
    pub(crate) base_color: Option<Buffer>,
    pub(crate) base_color_texture: Option<Texture>,
}

pub fn prepare_computed_meshes(
    device: Res<RenderDevice>,
    mut meshes: Query<(&Mesh, &mut ComputedMesh), Or<(Changed<Mesh>, Added<Mesh>)>>,
) {
    for (mesh, mut computed) in &mut meshes {
        let vertices = device.0.create_buffer_init(&BufferInitDescriptor {
            label: Some("mesh_vertex_buffer"),
            contents: bytemuck::cast_slice(&mesh.vertices()),
            usage: BufferUsages::VERTEX,
        });

        let indicies = mesh.indicies().unwrap().into_u32();
        let num_vertices = indicies.len() as u32;

        let indicies = device.0.create_buffer_init(&BufferInitDescriptor {
            label: Some("mesh_index_buffer"),
            contents: bytemuck::cast_slice(&indicies),
            usage: BufferUsages::INDEX,
        });

        computed.vertices = Some(Arc::new(vertices));
        computed.indicies = Some(Arc::new(indicies));
        computed.num_vertices = num_vertices;
    }
}
