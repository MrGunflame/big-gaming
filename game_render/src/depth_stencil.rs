use glam::UVec2;
use wgpu::{
    AddressMode, CompareFunction, Device, Extent3d, FilterMode, Sampler, SamplerDescriptor,
    Texture, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages, TextureView,
    TextureViewDescriptor,
};

pub(crate) const DEPTH_TEXTURE_FORMAT: TextureFormat = TextureFormat::Depth32Float;

#[derive(Debug)]
pub struct DepthData {
    pub texture: Texture,
    pub view: TextureView,
    pub sampler: Sampler,
}

impl DepthData {
    pub fn new(device: &Device, size: UVec2) -> Self {
        let size = Extent3d {
            width: size.x,
            height: size.y,
            depth_or_array_layers: 1,
        };

        let desc = TextureDescriptor {
            label: Some("depth_texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: DEPTH_TEXTURE_FORMAT,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };

        let texture = device.create_texture(&desc);

        let view = texture.create_view(&TextureViewDescriptor::default());

        let sampler = device.create_sampler(&SamplerDescriptor {
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Nearest,
            compare: Some(CompareFunction::LessEqual),
            lod_min_clamp: 0.0,
            lod_max_clamp: 100.0,
            ..Default::default()
        });

        Self {
            texture,
            view,
            sampler,
        }
    }
}
