use bevy_ecs::prelude::{Bundle, Res};
use bevy_ecs::system::{Query, ResMut, Resource};
use bevy_ecs::world::{FromWorld, World};
use game_asset::{Asset, Assets, Handle};
use game_common::bundles::TransformBundle;
use game_common::components::transform::Transform;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindingResource, Buffer, BufferUsages,
    Extent3d, ImageCopyTexture, ImageDataLayout, Origin3d, TextureAspect, TextureDescriptor,
    TextureDimension, TextureFormat, TextureUsages, TextureViewDescriptor,
};

use crate::color::Color;
use crate::mesh::Mesh;
use crate::pipeline::{MaterialPipeline, MeshPipeline};
use crate::texture::{Image, ImageHandle, Images};
use crate::{RenderDevice, RenderQueue};

#[derive(Resource)]
pub struct PbrResources {
    default_base_color_texture: ImageHandle,
    default_metallic_roughness_texture: ImageHandle,
}

impl PbrResources {
    pub fn new(images: &mut Images) -> Self {
        let default_base_color_texture = images.insert(Image {
            bytes: vec![255, 255, 255, 255],
            format: crate::texture::TextureFormat::Rgba8UnormSrgb,
            width: 1,
            height: 1,
        });

        let default_metallic_roughness_texture = images.insert(Image {
            bytes: vec![255, 255, 255, 255],
            format: crate::texture::TextureFormat::Rgba8UnormSrgb,
            width: 1,
            height: 1,
        });

        Self {
            default_base_color_texture,
            default_metallic_roughness_texture,
        }
    }
}

impl FromWorld for PbrResources {
    fn from_world(world: &mut World) -> Self {
        world.resource_scope::<Images, _>(|_, mut images| Self::new(&mut images))
    }
}

#[derive(Clone, Debug, Bundle)]
pub struct PbrBundle {
    pub mesh: Handle<Mesh>,
    pub material: Handle<PbrMaterial>,
    #[bundle]
    pub transform: TransformBundle,
}

#[derive(Clone, Debug)]
pub struct PbrMaterial {
    pub alpha_mode: AlphaMode,
    pub base_color: Color,
    pub base_color_texture: Option<ImageHandle>,

    pub roughness: f32,
    pub metallic: f32,
    pub metallic_roughness_texture: Option<ImageHandle>,
}

impl Default for PbrMaterial {
    fn default() -> Self {
        Self {
            alpha_mode: AlphaMode::default(),
            base_color: Color([1.0, 1.0, 1.0, 1.0]),
            base_color_texture: None,
            roughness: 0.0,
            metallic: 0.0,
            metallic_roughness_texture: None,
        }
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum AlphaMode {
    #[default]
    Opaque,
    Mask,
    Blend,
}

impl Asset for PbrMaterial {}

#[derive(Resource, Default)]
pub struct RenderMaterialAssets {
    pub entities: Vec<RenderNode>,
}

pub struct RenderNode {
    pub vertices: Buffer,
    pub indices: Buffer,
    pub num_vertices: u32,
    pub transform: Transform,
    pub bind_groups: Vec<BindGroup>,
}

pub fn prepare_materials(
    images: ResMut<Images>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    nodes: Query<(&Handle<Mesh>, &Handle<PbrMaterial>, &Transform)>,
    meshes: ResMut<Assets<Mesh>>,
    materials: ResMut<Assets<PbrMaterial>>,
    mut render_assets: ResMut<RenderMaterialAssets>,
    material_pipeline: Res<MaterialPipeline>,
    mesh_pipeline: Res<MeshPipeline>,
    pbr_res: Res<PbrResources>,
) {
    render_assets.entities.clear();

    for (mesh, material, transform) in &nodes {
        let Some(mesh) = meshes.get(mesh.id()) else {
            continue;
        };

        let Some(material) = materials.get(material.id()) else {
            continue;
        };

        let transform_buffer = device.0.create_buffer_init(&BufferInitDescriptor {
            label: Some("mesh_transform"),
            contents: bytemuck::cast_slice(&[transform.compute_matrix()]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let mesh_bind_group = device.0.create_bind_group(&BindGroupDescriptor {
            label: Some("mesh_bind_group"),
            layout: &mesh_pipeline.bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: mesh_pipeline.camera_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: transform_buffer.as_entire_binding(),
                },
            ],
        });

        let base_color = device.0.create_buffer_init(&BufferInitDescriptor {
            label: Some("pbr_material_base_color"),
            contents: bytemuck::cast_slice(&[material.base_color]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let material_base_color_texture = {
            let handle = material
                .base_color_texture
                .as_ref()
                .unwrap_or(&pbr_res.default_base_color_texture);

            images.get(handle).unwrap()
        };

        let size = Extent3d {
            width: material_base_color_texture.width,
            height: material_base_color_texture.height,
            depth_or_array_layers: 1,
        };

        let base_color_texture = device.0.create_texture(&TextureDescriptor {
            label: Some("pbr_material_base_color_texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8UnormSrgb,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.0.write_texture(
            ImageCopyTexture {
                texture: &base_color_texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            &material_base_color_texture.bytes,
            ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * material_base_color_texture.width),
                rows_per_image: Some(material_base_color_texture.height),
            },
            size,
        );

        let base_color_texture_view =
            base_color_texture.create_view(&TextureViewDescriptor::default());

        let material_bind_group = device.0.create_bind_group(&BindGroupDescriptor {
            label: Some("material_bind_group"),
            layout: &material_pipeline.bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: base_color.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&base_color_texture_view),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Sampler(&material_pipeline.sampler),
                },
            ],
        });

        let vertices = device.0.create_buffer_init(&BufferInitDescriptor {
            label: Some("mesh_vertex_buffer"),
            contents: bytemuck::cast_slice(&mesh.vertices()),
            usage: BufferUsages::VERTEX,
        });

        let indices = mesh.indicies().unwrap().into_u32();
        let num_vertices = indices.len() as u32;

        let indices = device.0.create_buffer_init(&BufferInitDescriptor {
            label: Some("mesh_index_buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: BufferUsages::INDEX,
        });

        render_assets.entities.push(RenderNode {
            vertices,
            indices,
            num_vertices,
            transform: *transform,
            bind_groups: vec![mesh_bind_group, material_bind_group],
        });
    }
}
