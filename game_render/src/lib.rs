#![deny(unsafe_op_in_unsafe_fn)]
#![deny(unused_crate_dependencies)]

pub mod aabb;
pub mod buffer;
pub mod camera;
pub mod color;
pub mod forward;
pub mod graph;
pub mod light;
pub mod mesh;
pub mod metrics;
pub mod mipmap;
pub mod pbr;
pub mod render_pass;
pub mod shape;
pub mod surface;
pub mod texture;

mod depth_stencil;
mod post_process;

use std::ops::Deref;

use bevy_app::{App, CoreSet, Plugin};
use bevy_ecs::schedule::{IntoSystemConfig, IntoSystemSetConfig, SystemSet};
use bevy_ecs::system::Resource;
use game_asset::AssetAppExt;
use game_core::hierarchy::HierarchyPlugin;
use game_core::transform::{TransformPlugin, TransformSet};
use game_window::WindowPlugin;
use graph::RenderGraph;
use mesh::Mesh;
use metrics::RenderMetrics;
use pbr::PbrMaterial;
use post_process::PostProcessPipeline;
use render_pass::{RenderNodes, RenderPass};
use surface::RenderSurfaces;
use texture::ImagePlugin;
use wgpu::{
    Adapter, Backends, Device, DeviceDescriptor, Features, Instance, InstanceDescriptor, Limits,
    PowerPreference, Queue, RequestAdapterOptions,
};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, SystemSet)]
pub enum RenderSet {
    UpdateSurfaces,
    Update,
    Render,
}

#[derive(Copy, Clone, Debug, Default)]
pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(HierarchyPlugin);
        app.add_plugin(TransformPlugin);

        app.add_plugin(WindowPlugin);

        app.add_plugin(ImagePlugin);

        let instance = Instance::new(InstanceDescriptor {
            backends: Backends::VULKAN,
            dx12_shader_compiler: Default::default(),
        });

        let adapter =
            futures_lite::future::block_on(instance.request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::default(),
                compatible_surface: None,
                force_fallback_adapter: false,
            }))
            .unwrap();

        let features = Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES;

        let (device, queue) = futures_lite::future::block_on(adapter.request_device(
            &DeviceDescriptor {
                features,
                limits: Limits::default(),
                label: None,
            },
            None,
        ))
        .unwrap();

        app.add_asset::<Mesh>();
        app.add_asset::<PbrMaterial>();

        app.insert_resource(RenderInstance(instance));
        app.insert_resource(RenderAdapter(adapter));
        app.insert_resource(RenderDevice(device));
        app.insert_resource(RenderQueue(queue));

        let mut render_graph = RenderGraph::default();
        render_graph.push(RenderPass);
        app.insert_resource(render_graph);

        // Surface configuration
        {
            app.insert_resource(RenderSurfaces::default());
            app.add_system(surface::create_surfaces.in_set(RenderSet::UpdateSurfaces));
            app.add_system(surface::destroy_surfaces.in_set(RenderSet::UpdateSurfaces));
            app.add_system(
                surface::resize_surfaces
                    .in_set(RenderSet::UpdateSurfaces)
                    .after(surface::create_surfaces),
            );

            app.add_system(surface::render_to_surfaces.in_set(RenderSet::Render));
        }

        // Pipelines
        {
            app.init_resource::<forward::ForwardPipeline>();

            app.insert_resource(depth_stencil::DepthTextures::default());
            app.add_system(depth_stencil::create_depth_textures.in_set(RenderSet::Update));
            app.add_system(depth_stencil::resize_depth_textures.in_set(RenderSet::Update));
            app.add_system(depth_stencil::destroy_depth_textures.in_set(RenderSet::Update));

            app.insert_resource(light::pipeline::DirectionalLightCache::default());
            app.insert_resource(light::pipeline::PointLightCache::default());
            app.insert_resource(light::pipeline::SpotLightCache::default());
            app.add_system(light::pipeline::update_directional_lights);
            app.add_system(light::pipeline::update_point_lights);
            app.add_system(light::pipeline::update_spot_lights);
        }

        // Post Process Pipeline
        {
            app.init_resource::<PostProcessPipeline>();
        }

        // PBR
        {
            app.init_resource::<RenderNodes>();
            app.add_system(pbr::mesh::update_mesh_bind_group);
            app.add_system(pbr::mesh::update_mesh_transform);

            app.init_resource::<pbr::material::DefaultTextures>();
            app.add_system(pbr::material::update_material_bind_groups);
        }

        // Camera
        {
            app.insert_resource(camera::Cameras::default());
            app.add_system(camera::create_cameras.in_set(RenderSet::Update));
            app.add_system(camera::update_camera_aspect_ratio.in_set(RenderSet::Update));
            app.add_system(
                camera::update_camera_buffer
                    .in_set(RenderSet::Update)
                    .after(camera::update_camera_aspect_ratio),
            );
        }

        // Mipmap
        {
            app.init_resource::<mipmap::MipMapGenerator>();
        }

        app.configure_set(
            RenderSet::Render
                .after(RenderSet::Update)
                .in_base_set(CoreSet::PostUpdate),
        );
        app.configure_set(
            RenderSet::Update
                .after(RenderSet::UpdateSurfaces)
                .in_base_set(CoreSet::PostUpdate),
        );
        app.configure_set(
            RenderSet::UpdateSurfaces
                .after(TransformSet)
                .in_base_set(CoreSet::PostUpdate),
        );

        app.add_system(aabb::update_aabb);

        app.insert_resource(RenderMetrics::default());
    }
}

#[derive(Debug, Resource)]
pub struct RenderInstance(pub Instance);

impl Deref for RenderInstance {
    type Target = Instance;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Resource)]
pub struct RenderAdapter(pub Adapter);

impl Deref for RenderAdapter {
    type Target = Adapter;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Resource)]
pub struct RenderDevice(pub Device);

impl Deref for RenderDevice {
    type Target = Device;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Resource)]
pub struct RenderQueue(pub Queue);

impl Deref for RenderQueue {
    type Target = Queue;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
