use std::collections::HashMap;

use bitflags::bitflags;
use bytemuck::{Pod, Zeroable};
use game_tracing::trace_span;
use glam::UVec2;
use slotmap::{DefaultKey, SlotMap};
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindingResource, BufferUsages,
    CommandEncoderDescriptor, Device, Extent3d, ImageCopyTexture, ImageDataLayout, Origin3d, Queue,
    TextureAspect, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages, TextureView,
    TextureViewDescriptor,
};

use crate::forward::ForwardPipeline;
use crate::mipmap::MipMapGenerator;
use crate::texture::{Image, ImageId, Images};

use super::PbrMaterial;

bitflags! {
    #[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, Zeroable, Pod)]
    #[repr(transparent)]
    pub struct MaterialFlags: u32 {
        const UNLIT = 0b0000_0000_0000_0001;
    }
}

#[derive(Clone, Debug)]
pub struct DefaultTextures {
    pub default_base_color_texture: ImageId,
    pub default_normal_texture: ImageId,
    pub default_metallic_roughness_texture: ImageId,
}

impl DefaultTextures {
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

pub fn update_material_bind_group(
    device: &Device,
    queue: &Queue,
    images: &HashMap<ImageId, Image>,
    pipeline: &ForwardPipeline,
    material: &PbrMaterial,
    mipmap_generator: &mut MipMapGenerator,
) -> BindGroup {
    let _span = trace_span!("update_material_bind_group").entered();

    let default_textures = &pipeline.default_textures;

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
            .unwrap_or(default_textures.default_base_color_texture),
        images,
        &device,
        &queue,
        mipmap_generator,
    );

    let normal_texture = create_material_texture(
        material
            .normal_texture
            .unwrap_or(default_textures.default_normal_texture),
        images,
        &device,
        &queue,
        mipmap_generator,
    );

    let metallic_roughness_texture = create_material_texture(
        material
            .metallic_roughness_texture
            .unwrap_or(default_textures.default_metallic_roughness_texture),
        images,
        &device,
        &queue,
        mipmap_generator,
    );

    device.create_bind_group(&BindGroupDescriptor {
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
    })
}

fn create_material_texture(
    id: ImageId,
    images: &HashMap<ImageId, Image>,
    device: &Device,
    queue: &Queue,
    mipmap_generator: &mut MipMapGenerator,
) -> TextureView {
    let _span = trace_span!("create_material_texture").entered();

    let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor { label: None });

    let data = images.get(&id).unwrap();

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

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct MaterialId(DefaultKey);

pub struct Materials {
    materials: SlotMap<DefaultKey, PbrMaterial>,
}

impl Materials {
    pub fn new() -> Self {
        Self {
            materials: SlotMap::new(),
        }
    }

    pub fn insert(&mut self, material: PbrMaterial) -> MaterialId {
        let key = self.materials.insert(material);
        MaterialId(key)
    }

    pub fn remove(&mut self, id: MaterialId) {
        self.materials.remove(id.0);
    }

    pub fn get(&self, id: MaterialId) -> Option<&PbrMaterial> {
        self.materials.get(id.0)
    }
}
