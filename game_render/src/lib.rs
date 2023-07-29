#![deny(unsafe_op_in_unsafe_fn)]
#![deny(unused_crate_dependencies)]

pub mod aabb;
pub mod buffer;
pub mod camera;
pub mod color;
pub mod forward;
pub mod graph;
pub mod light;
pub mod material;
pub mod mesh;
pub mod pbr;
pub mod pipeline;
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
use pbr::{PbrMaterial, RenderMaterialAssets};
use pipeline::{LightingPipeline, MainPass};
use post_process::PostProcessPipeline;
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

        app.init_resource::<pbr::PbrResources>();

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
        app.insert_resource(RenderMaterialAssets::default());

        app.insert_resource(RenderInstance(instance));
        app.insert_resource(RenderAdapter(adapter));
        app.insert_resource(RenderDevice(device));
        app.insert_resource(RenderQueue(queue));

        let mut render_graph = RenderGraph::default();
        render_graph.push(MainPass::default());
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

        app.init_resource::<pipeline::MeshPipeline>();
        app.init_resource::<pipeline::MaterialPipeline>();
        app.init_resource::<LightingPipeline>();

        app.insert_resource(camera::Cameras::default());
        app.add_system(camera::create_cameras.in_set(RenderSet::Update));
        app.add_system(camera::update_camera_aspect_ratio.in_set(RenderSet::Update));
        app.add_system(
            camera::update_camera_buffer
                .in_set(RenderSet::Update)
                .after(camera::update_camera_aspect_ratio),
        );

        app.add_system(pbr::prepare_materials.in_set(RenderSet::Update));
        app.add_system(pbr::prepare_directional_lights.in_set(RenderSet::Update));
        app.add_system(
            pbr::update_material_bind_groups
                .after(pbr::prepare_materials)
                .in_set(RenderSet::Update),
        );
        app.add_system(pbr::prepare_point_lights.in_set(RenderSet::Update));
        app.add_system(pbr::remove_render_nodes.in_set(RenderSet::Update));

        app.insert_resource(pipeline::RenderWindows::default());
        app.add_system(pipeline::create_render_windows);
        app.add_system(pipeline::destroy_render_windows);
        app.add_system(pipeline::resize_render_windows);

        app.init_resource::<PostProcessPipeline>();

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
