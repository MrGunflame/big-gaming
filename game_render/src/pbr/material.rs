use bevy_ecs::prelude::Entity;
use bevy_ecs::query::{Added, Changed, Or};
use bevy_ecs::system::{Query, Res, ResMut, Resource};
use bevy_ecs::world::FromWorld;
use bitflags::bitflags;
use bytemuck::{Pod, Zeroable};
use game_asset::{Assets, Handle};
use glam::UVec2;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{
    BindGroupDescriptor, BindGroupEntry, BindingResource, BufferUsages, CommandEncoderDescriptor,
    Device, Extent3d, ImageCopyTexture, ImageDataLayout, Origin3d, Queue, TextureAspect,
    TextureDescriptor, TextureDimension, TextureFormat, TextureUsages, TextureView,
    TextureViewDescriptor,
};

use crate::forward::ForwardPipeline;
use crate::mipmap::MipMapGenerator;
use crate::render_pass::RenderNodes;
use crate::texture::{Image, ImageHandle, Images};
use crate::{RenderDevice, RenderQueue};

use super::PbrMaterial;

bitflags! {
    #[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, Zeroable, Pod)]
    #[repr(transparent)]
    pub struct MaterialFlags: u32 {
        const UNLIT = 0b0000_0000_0000_0001;
    }
}

#[derive(Clone, Debug, Resource)]
pub struct DefaultTextures {
    default_base_color_texture: ImageHandle,
    default_normal_texture: ImageHandle,
    default_metallic_roughness_texture: ImageHandle,
}

impl DefaultTextures {
    fn new(images: &mut Images) -> Self {
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

impl FromWorld for DefaultTextures {
    fn from_world(world: &mut bevy_ecs::world::World) -> Self {
        world.resource_scope::<Images, _>(|_, mut images| Self::new(&mut images))
    }
}

pub fn update_material_bind_groups(
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    materials: Res<Assets<PbrMaterial>>,
    pipeline: Res<ForwardPipeline>,
    nodes: Query<
        (Entity, &Handle<PbrMaterial>),
        Or<(Added<Handle<PbrMaterial>>, Changed<Handle<PbrMaterial>>)>,
    >,
    mut render_nodes: ResMut<RenderNodes>,
    default_textures: Res<DefaultTextures>,
    images: Res<Images>,
    mut mipmap_generator: ResMut<MipMapGenerator>,
) {
    for (entity, handle) in &nodes {
        let Some(material) = materials.get(handle.id()) else {
            continue;
        };

        let constants = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("material_constants"),
            contents: bytemuck::cast_slice(&[MaterialConstants {
                base_color: material.base_color.0,
                base_metallic: material.metallic,
                base_roughness: material.roughness,
                _pad: [0; 2],
            }]),
            usage: BufferUsages::UNIFORM,
        });

        let base_color_texture = create_material_texture(
            material
                .base_color_texture
                .as_ref()
                .unwrap_or(&default_textures.default_base_color_texture),
            &images,
            &device,
            &queue,
            &mut mipmap_generator,
        );

        let normal_texture = create_material_texture(
            material
                .normal_texture
                .as_ref()
                .unwrap_or(&default_textures.default_normal_texture),
            &images,
            &device,
            &queue,
            &mut mipmap_generator,
        );

        let metallic_roughness_texture = create_material_texture(
            material
                .metallic_roughness_texture
                .as_ref()
                .unwrap_or(&default_textures.default_metallic_roughness_texture),
            &images,
            &device,
            &queue,
            &mut mipmap_generator,
        );

        let material_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("material_bind_group"),
            layout: &pipeline.material_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: constants.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&base_color_texture),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(&normal_texture),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::TextureView(&metallic_roughness_texture),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: BindingResource::Sampler(&pipeline.sampler),
                },
            ],
        });

        let node = render_nodes.entities.entry(entity).or_default();
        node.material_bind_group = Some(material_bind_group);
    }
}

fn create_material_texture(
    handle: &ImageHandle,
    images: &Images,
    device: &Device,
    queue: &Queue,
    mipmap_generator: &mut MipMapGenerator,
) -> TextureView {
    let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor { label: None });

    let data = images.get(handle).unwrap();

    let size = Extent3d {
        width: data.width(),
        height: data.height(),
        depth_or_array_layers: 1,
    };

    let texture = device.create_texture(&TextureDescriptor {
        label: None,
        size,
        mip_level_count: size.max_mips(TextureDimension::D2),
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: data.format(),
        usage: TextureUsages::TEXTURE_BINDING
            | TextureUsages::COPY_DST
            | TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });

    queue.write_texture(
        ImageCopyTexture {
            texture: &texture,
            mip_level: 0,
            origin: Origin3d::ZERO,
            aspect: TextureAspect::All,
        },
        data.as_bytes(),
        ImageDataLayout {
            offset: 0,
            // TODO: Support for non-RGBA (non 4 px) textures.
            bytes_per_row: Some(4 * data.width()),
            rows_per_image: Some(data.height()),
        },
        size,
    );

    mipmap_generator.generate_mipmaps(device, &mut encoder, &texture);

    queue.submit(std::iter::once(encoder.finish()));

    texture.create_view(&TextureViewDescriptor::default())
}

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
pub struct MaterialConstants {
    pub base_color: [f32; 4],
    pub base_metallic: f32,
    pub base_roughness: f32,
    // Align to vec4<f32>.
    pub _pad: [u32; 2],
}
