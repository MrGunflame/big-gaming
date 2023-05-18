use std::sync::Arc;

use bevy_ecs::prelude::{Bundle, Component, Res};
use bevy_ecs::query::{Added, Changed, Or};
use bevy_ecs::system::Query;
use image::{ImageBuffer, Rgba};
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{
    Buffer, BufferUsages, Extent3d, ImageCopyTexture, ImageDataLayout, Origin3d, Texture,
    TextureAspect, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
};

use crate::mesh::Mesh;
use crate::{RenderDevice, RenderQueue};

#[derive(Debug, Bundle)]
pub struct MaterialMeshBundle {
    pub mesh: Mesh,
    pub material: Material,
    pub computed_material: ComputedMaterial,
    pub computed_mesh: ComputedMesh,
}

#[derive(Clone, Debug, Component)]
pub struct Material {
    pub color: [f32; 4],
    pub color_texture: ImageBuffer<Rgba<u8>, Vec<u8>>,
}

impl Default for Material {
    fn default() -> Self {
        let mut color_texture: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::new(1, 1);
        color_texture.fill(255);
        // color_texture[(0, 0)] = Rgba([255, 255, 255, 255]);

        Self {
            color: [1.0, 1.0, 1.0, 1.0],
            color_texture,
        }
    }
}

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

        let indicies = mesh.indicies().unwrap();
        let num_vertices = indicies.len() as u32;

        let indicies = device.0.create_buffer_init(&BufferInitDescriptor {
            label: Some("mesh_index_buffer"),
            contents: bytemuck::cast_slice(indicies.as_u32()),
            usage: BufferUsages::INDEX,
        });

        computed.vertices = Some(Arc::new(vertices));
        computed.indicies = Some(Arc::new(indicies));
        computed.num_vertices = num_vertices;
    }
}

pub fn prepare_computed_materials(
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    mut materials: Query<
        (&Material, &mut ComputedMaterial),
        Or<(Changed<Material>, Added<Material>)>,
    >,
) {
    for (material, mut computed) in &mut materials {
        let base_color = device.0.create_buffer_init(&BufferInitDescriptor {
            label: Some("base_color_buffer"),
            contents: bytemuck::cast_slice(&[material.color]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        computed.base_color = Some(base_color);

        let size = Extent3d {
            width: material.color_texture.width(),
            height: material.color_texture.height(),
            depth_or_array_layers: 1,
        };

        let texture = device.0.create_texture(&TextureDescriptor {
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8UnormSrgb,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            label: Some("base_color_texture"),
            view_formats: &[],
        });

        queue.0.write_texture(
            ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            &material.color_texture,
            ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * material.color_texture.width()),
                rows_per_image: Some(material.color_texture.height()),
            },
            size,
        );

        computed.base_color_texture = Some(texture);
    }
}
