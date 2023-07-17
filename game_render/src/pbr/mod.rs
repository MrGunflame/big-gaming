use std::collections::HashMap;

use bevy_ecs::prelude::{Bundle, Entity, Res};
use bevy_ecs::query::{Added, Changed, Or};
use bevy_ecs::removal_detection::RemovedComponents;
use bevy_ecs::system::{Query, ResMut, Resource};
use bevy_ecs::world::{FromWorld, World};
use game_asset::{Asset, Assets, Handle};
use game_common::bundles::TransformBundle;
use game_common::components::transform::{GlobalTransform, Transform};
use glam::UVec2;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindingResource, Buffer, BufferUsages, Device,
    Extent3d, ImageCopyTexture, ImageDataLayout, IndexFormat, Origin3d, Queue, TextureAspect,
    TextureDescriptor, TextureDimension, TextureFormat, TextureUsages, TextureView,
    TextureViewDescriptor,
};

use crate::buffer::IndexBuffer;
use crate::color::Color;
use crate::light::pipeline::{DirectionalLightUniform, PointLightUniform};
use crate::light::{DirectionalLight, PointLight};
use crate::mesh::{Indices, Mesh};
use crate::pipeline::{LightingPipeline, MaterialPipeline, MeshPipeline, TransformUniform};
use crate::texture::{Image, ImageHandle, Images};
use crate::{RenderDevice, RenderQueue};

#[derive(Resource)]
pub struct PbrResources {
    default_base_color_texture: ImageHandle,
    default_normal_texture: ImageHandle,
    default_metallic_roughness_texture: ImageHandle,
}

impl PbrResources {
    pub fn new(images: &mut Images) -> Self {
        let default_base_color_texture = images.insert(Image::new(
            UVec2::splat(1),
            TextureFormat::Rgba8UnormSrgb,
            vec![255, 255, 255, 255],
        ));

        // B channel facing towards local Z.
        let default_normal_texture = images.insert(Image::new(
            UVec2::splat(1),
            TextureFormat::Rgba8Unorm,
            vec![(0.5 * 255.0) as u8, (0.5 * 255.0) as u8, 255, 255],
        ));

        let default_metallic_roughness_texture = images.insert(Image::new(
            UVec2::splat(1),
            TextureFormat::Rgba8UnormSrgb,
            vec![255, 255, 255, 255],
        ));

        Self {
            default_base_color_texture,
            default_normal_texture,
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

    pub normal_texture: Option<ImageHandle>,

    pub roughness: f32,
    pub metallic: f32,
    pub metallic_roughness_texture: Option<ImageHandle>,
}

impl Default for PbrMaterial {
    fn default() -> Self {
        Self {
            alpha_mode: AlphaMode::default(),
            base_color: Color::WHITE,
            base_color_texture: None,
            normal_texture: None,
            roughness: 0.5,
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
    pub entities: HashMap<Entity, RenderNode>,
    pub directional_lights: Vec<DirectionalLightNode>,
    pub point_lights: Vec<PointLightNode>,
}

pub struct RenderNode {
    pub vertices: Buffer,
    pub indices: IndexBuffer,
    pub transform: Transform,
    pub transform_buffer: Buffer,
    pub transform_bind_group: BindGroup,
    pub material_bind_group: Option<BindGroup>,
}

pub struct DirectionalLightNode {
    pub bind_group: BindGroup,
}

pub struct PointLightNode {
    pub bind_group: BindGroup,
}

pub fn remove_render_nodes(
    mut res: ResMut<RenderMaterialAssets>,
    mut materials: RemovedComponents<Handle<PbrMaterial>>,
    mut meshes: RemovedComponents<Handle<Mesh>>,
) {
    for entity in meshes.iter() {
        res.entities.remove(&entity);
    }

    for entity in materials.iter() {
        res.entities.remove(&entity);
    }
}

pub fn update_material_bind_groups(
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    materials: Res<Assets<PbrMaterial>>,
    material_pipeline: Res<MaterialPipeline>,
    images: Res<Images>,
    nodes: Query<
        (
            Entity,
            &Handle<Mesh>,
            &Handle<PbrMaterial>,
            &GlobalTransform,
        ),
        Or<(Changed<Handle<PbrMaterial>>, Added<Handle<PbrMaterial>>)>,
    >,
    pbr_res: Res<PbrResources>,
    mut render_assets: ResMut<RenderMaterialAssets>,
) {
    for (entity, mesh, material, transform) in &nodes {
        let Some(material) = materials.get(material.id()) else {
            continue;
        };

        let base_color = device.0.create_buffer_init(&BufferInitDescriptor {
            label: Some("pbr_material_base_color"),
            contents: bytemuck::cast_slice(&[material.base_color]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let roughness = device.0.create_buffer_init(&BufferInitDescriptor {
            label: Some("pbr_material_roughness"),
            contents: bytemuck::cast_slice(&[material.roughness]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let metallic = device.0.create_buffer_init(&BufferInitDescriptor {
            label: Some("pbr_material_metallic"),
            contents: bytemuck::cast_slice(&[material.metallic]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let base_color_texture = setup_render_texture(
            material
                .base_color_texture
                .as_ref()
                .unwrap_or(&pbr_res.default_base_color_texture),
            &images,
            &device.0,
            &queue.0,
        );

        let normal_texture = setup_render_texture(
            material
                .normal_texture
                .as_ref()
                .unwrap_or(&pbr_res.default_normal_texture),
            &images,
            &device.0,
            &queue.0,
        );

        let metallic_roughness_texture = setup_render_texture(
            material
                .metallic_roughness_texture
                .as_ref()
                .unwrap_or(&pbr_res.default_metallic_roughness_texture),
            &images,
            &device.0,
            &queue.0,
        );

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
                    resource: BindingResource::TextureView(&base_color_texture),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Sampler(&material_pipeline.sampler),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::TextureView(&normal_texture),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: BindingResource::Sampler(&material_pipeline.sampler),
                },
                BindGroupEntry {
                    binding: 5,
                    resource: roughness.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 6,
                    resource: metallic.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 7,
                    resource: BindingResource::TextureView(&metallic_roughness_texture),
                },
                BindGroupEntry {
                    binding: 8,
                    resource: BindingResource::Sampler(&material_pipeline.sampler),
                },
            ],
        });

        render_assets
            .entities
            .get_mut(&entity)
            .unwrap()
            .material_bind_group = Some(material_bind_group);
    }
}

pub fn prepare_materials(
    device: Res<RenderDevice>,
    nodes: Query<(
        Entity,
        &Handle<Mesh>,
        &Handle<PbrMaterial>,
        &GlobalTransform,
    )>,
    meshes: Res<Assets<Mesh>>,
    mut render_assets: ResMut<RenderMaterialAssets>,
    mesh_pipeline: Res<MeshPipeline>,
) {
    for (entity, mesh, material, transform) in &nodes {
        let Some(mesh) = meshes.get(mesh.id()) else {
            continue;
        };

        let transform_buffer = device.0.create_buffer_init(&BufferInitDescriptor {
            label: Some("mesh_transform"),
            contents: bytemuck::cast_slice(&[TransformUniform::from(transform.0)]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let transform_bind_group = device.0.create_bind_group(&BindGroupDescriptor {
            label: Some("mesh_bind_group"),
            layout: &mesh_pipeline.model_bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: transform_buffer.as_entire_binding(),
            }],
        });

        let vertices = device.0.create_buffer_init(&BufferInitDescriptor {
            label: Some("mesh_vertex_buffer"),
            contents: bytemuck::cast_slice(&mesh.vertices()),
            usage: BufferUsages::VERTEX,
        });

        let indices = match mesh.indicies().unwrap() {
            Indices::U16(buf) => {
                let buffer = device.0.create_buffer_init(&BufferInitDescriptor {
                    label: Some("mesh_index_buffer"),
                    contents: bytemuck::cast_slice(&buf),
                    usage: BufferUsages::INDEX,
                });

                IndexBuffer {
                    buffer,
                    format: IndexFormat::Uint16,
                    len: buf.len() as u32,
                }
            }
            Indices::U32(buf) => {
                let buffer = device.0.create_buffer_init(&BufferInitDescriptor {
                    label: Some("mesh_index_buffer"),
                    contents: bytemuck::cast_slice(&buf),
                    usage: BufferUsages::INDEX,
                });

                IndexBuffer {
                    buffer,
                    format: IndexFormat::Uint32,
                    len: buf.len() as u32,
                }
            }
        };

        match render_assets.entities.get_mut(&entity) {
            Some(node) => {
                node.vertices = vertices;
                node.indices = indices;
                node.transform = transform.0;
                node.transform_buffer = transform_buffer;
                node.transform_bind_group = transform_bind_group;
            }
            None => {
                render_assets.entities.insert(
                    entity,
                    RenderNode {
                        vertices,
                        indices,
                        transform: transform.0,
                        transform_buffer,
                        transform_bind_group,
                        material_bind_group: None,
                    },
                );
            }
        }
    }
}

fn setup_render_texture(
    handle: &ImageHandle,
    images: &Images,
    device: &Device,
    queue: &Queue,
) -> TextureView {
    let texture_data = images.get(handle).unwrap();

    let size = Extent3d {
        width: texture_data.width(),
        height: texture_data.height(),
        depth_or_array_layers: 1,
    };

    let texture = device.create_texture(&TextureDescriptor {
        label: None,
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: texture_data.format(),
        usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
        view_formats: &[],
    });

    queue.write_texture(
        ImageCopyTexture {
            texture: &texture,
            mip_level: 0,
            origin: Origin3d::ZERO,
            aspect: TextureAspect::All,
        },
        texture_data.as_bytes(),
        ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(4 * texture_data.width()),
            rows_per_image: Some(texture_data.height()),
        },
        size,
    );

    texture.create_view(&TextureViewDescriptor::default())
}

pub fn prepare_directional_lights(
    mut render_assets: ResMut<RenderMaterialAssets>,
    lights: Query<(&DirectionalLight, &GlobalTransform)>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    pipeline: Res<LightingPipeline>,
) {
    render_assets.directional_lights.clear();

    for (light, transform) in &lights {
        let uniform = DirectionalLightUniform {
            color: light.color,
            position: transform.0.translation.to_array(),
            _pad0: 0,
            _pad1: 0,
        };

        let buffer = device.0.create_buffer_init(&BufferInitDescriptor {
            label: Some("directional_light_buffer"),
            contents: bytemuck::cast_slice(&[uniform]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let bind_group = device.0.create_bind_group(&BindGroupDescriptor {
            label: Some("directional_light_bind_group"),
            layout: &pipeline.light_bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });

        render_assets
            .directional_lights
            .push(DirectionalLightNode { bind_group });
    }
}

pub fn prepare_point_lights(
    mut render_assets: ResMut<RenderMaterialAssets>,
    lights: Query<(&PointLight, &GlobalTransform)>,
    device: Res<RenderDevice>,
    pipeline: Res<LightingPipeline>,
) {
    render_assets.point_lights.clear();

    for (light, transform) in &lights {
        let uniform = PointLightUniform {
            color: light.color.rgb(),
            position: transform.0.translation.to_array(),
            _pad0: 0,
            _pad1: 0,
        };

        let buffer = device.0.create_buffer_init(&BufferInitDescriptor {
            label: Some("point_light_buffer"),
            contents: bytemuck::cast_slice(&[uniform]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let bind_group = device.0.create_bind_group(&BindGroupDescriptor {
            label: Some("point_light_bind_group"),
            layout: &pipeline.light_bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });

        render_assets
            .point_lights
            .push(PointLightNode { bind_group });
    }
}
