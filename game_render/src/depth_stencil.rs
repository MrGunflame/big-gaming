use std::collections::HashMap;

use bevy_ecs::prelude::{Entity, EventReader};
use bevy_ecs::system::{Query, Res, ResMut, Resource};
use game_window::events::{WindowCreated, WindowDestroyed, WindowResized};
use game_window::WindowState;
use glam::UVec2;
use wgpu::{
    AddressMode, CompareFunction, Device, Extent3d, FilterMode, Sampler, SamplerDescriptor,
    Texture, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages, TextureView,
    TextureViewDescriptor,
};

use crate::RenderDevice;

pub(crate) const DEPTH_TEXTURE_FORMAT: TextureFormat = TextureFormat::Depth32Float;

pub(crate) fn create_depth_texture(
    device: &Device,
    dimensions: UVec2,
) -> (Texture, TextureView, Sampler) {
    let size = Extent3d {
        width: dimensions.x,
        height: dimensions.y,
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

    (texture, view, sampler)
}

#[derive(Debug, Default, Resource)]
pub struct DepthTextures {
    pub windows: HashMap<Entity, DepthData>,
}

#[derive(Debug)]
pub struct DepthData {
    pub texture: Texture,
    pub view: TextureView,
    pub sampler: Sampler,
}

pub fn create_depth_textures(
    mut textures: ResMut<DepthTextures>,
    device: Res<RenderDevice>,
    mut events: EventReader<WindowCreated>,
    windows: Query<&WindowState>,
) {
    for event in events.iter() {
        let window = windows.get(event.window).unwrap();
        let size = window.inner_size();

        let (texture, view, sampler) =
            create_depth_texture(&device, UVec2::new(size.width, size.height));

        textures.windows.insert(
            event.window,
            DepthData {
                texture,
                view,
                sampler,
            },
        );
    }
}

pub fn resize_depth_textures(
    mut textures: ResMut<DepthTextures>,
    device: Res<RenderDevice>,
    mut events: EventReader<WindowResized>,
) {
    for event in events.iter() {
        let window = textures.windows.get_mut(&event.window).unwrap();

        let (texture, view, sampler) =
            create_depth_texture(&device, UVec2::new(event.width, event.height));

        window.texture = texture;
        window.view = view;
        window.sampler = sampler;
    }
}

pub fn destroy_depth_textures(
    mut textures: ResMut<DepthTextures>,
    mut events: EventReader<WindowDestroyed>,
) {
    for event in events.iter() {
        textures.windows.remove(&event.window);
    }
}
