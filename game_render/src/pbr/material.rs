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
    Texture, TextureAspect, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
    TextureView, TextureViewDescriptor,
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

pub(crate) fn create_texture(
    image: &Image,
    device: &Device,
    queue: &Queue,
    mipmap_generator: &mut MipMapGenerator,
) -> Texture {
    let _span = trace_span!("create_material_texture").entered();

    let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor { label: None });

    let size = Extent3d {
        width: image.width(),
        height: image.height(),
        depth_or_array_layers: 1,
    };

    let texture = device.create_texture(&TextureDescriptor {
        label: None,
        size,
        mip_level_count: size.max_mips(TextureDimension::D2),
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: image.format(),
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
        image.as_bytes(),
        ImageDataLayout {
            offset: 0,
            // TODO: Support for non-RGBA (non 4 px) textures.
            bytes_per_row: Some(4 * image.width()),
            rows_per_image: Some(image.height()),
        },
        size,
    );

    mipmap_generator.generate_mipmaps(device, &mut encoder, &texture);

    queue.submit(std::iter::once(encoder.finish()));

    texture
}

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
pub struct MaterialConstants {
    pub base_color: [f32; 4],
    pub base_metallic: f32,
    pub base_roughness: f32,
    pub reflectance: f32,
    // Align to vec4<f32>.
    pub _pad: [u32; 1],
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

impl Default for Materials {
    fn default() -> Self {
        Self::new()
    }
}
