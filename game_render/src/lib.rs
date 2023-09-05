#![deny(unsafe_op_in_unsafe_fn)]
#![deny(unused_crate_dependencies)]

pub mod aabb;
pub mod buffer;
pub mod camera;
pub mod color;
pub mod entities;
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

use camera::RenderTarget;
use entities::SceneEntities;
use forward::ForwardPipeline;
use game_asset::Assets;
use game_window::windows::{WindowId, WindowState};
use glam::UVec2;
use graph::{RenderContext, RenderGraph};
use mesh::Mesh;
use mipmap::MipMapGenerator;
use pbr::PbrMaterial;
use post_process::PostProcessPipeline;
use render_pass::RenderPass;
use surface::RenderSurfaces;
use texture::Images;
use wgpu::{
    Adapter, Backends, CommandEncoderDescriptor, Device, DeviceDescriptor, Features, Instance,
    InstanceDescriptor, Limits, PowerPreference, Queue, RequestAdapterOptions,
    TextureViewDescriptor,
};

pub struct Renderer {
    instance: Instance,
    adapter: Adapter,
    pub device: Device,
    pub queue: Queue,
    pub graph: RenderGraph,
    pub surfaces: RenderSurfaces,
    pub entities: SceneEntities,
    pub images: Images,
    forward: ForwardPipeline,
    post_process: PostProcessPipeline,
    mipmap_generator: MipMapGenerator,
    pub meshes: Assets<Mesh>,
    pub materials: Assets<PbrMaterial>,
}

impl Renderer {
    pub fn new() -> Self {
        let instance = Instance::new(InstanceDescriptor {
            backends: Backends::VULKAN,
            dx12_shader_compiler: Default::default(),
        });

        let adapter =
            futures_lite::future::block_on(instance.request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            }))
            .unwrap();

        let (device, queue) = futures_lite::future::block_on(adapter.request_device(
            &DeviceDescriptor {
                features: Features::default(),
                limits: Limits::default(),
                label: None,
            },
            None,
        ))
        .unwrap();

        let mut graph = RenderGraph::default();
        graph.push(RenderPass);

        let mut images = Images::new();

        Self {
            entities: SceneEntities::new(&device),
            forward: ForwardPipeline::new(&device, &mut images),
            post_process: PostProcessPipeline::new(&device),
            mipmap_generator: MipMapGenerator::new(&device),
            instance,
            adapter,
            device,
            queue,
            graph,
            surfaces: RenderSurfaces::new(),
            images,
            materials: Assets::new(),
            meshes: Assets::new(),
        }
    }

    /// Create a new renderer for the window.
    pub fn create(&mut self, id: WindowId, window: WindowState) {
        self.surfaces
            .create(&self.instance, &self.adapter, &self.device, window, id);
    }

    pub fn resize(&mut self, id: WindowId, size: UVec2) {
        self.surfaces.resize(id, &self.device, size);

        for cam in self.entities.cameras().values_mut() {
            if cam.target == RenderTarget::Window(id) {
                cam.update_aspect_ratio(size);
            }
        }
    }

    pub fn destroy(&mut self, id: WindowId) {
        self.surfaces.destroy(id);
    }

    pub fn render(&mut self) {
        self.entities.rebuild(
            &mut self.meshes,
            &mut self.materials,
            &mut self.images,
            &self.device,
            &self.queue,
            &self.forward,
            &mut self.mipmap_generator,
        );

        // FIXME: Should update on render pass.
        crate::texture::image::load_images(&mut self.images);
        crate::texture::image::update_image_handles(&mut self.images);

        for (window, surface) in self.surfaces.iter_mut() {
            let output = match surface.surface.get_current_texture() {
                Ok(output) => output,
                Err(err) => {
                    tracing::error!("failed to get surface: {}", err);
                    continue;
                }
            };

            let target = output.texture.create_view(&TextureViewDescriptor {
                label: Some("surface_view"),
                format: Some(surface.config.format),
                ..Default::default()
            });

            let mut encoder = self
                .device
                .create_command_encoder(&CommandEncoderDescriptor {
                    label: Some("render_encoder"),
                });

            let mut ctx = RenderContext {
                state: &self.entities.state,
                window: *window,
                encoder: &mut encoder,
                width: output.texture.width(),
                height: output.texture.height(),
                format: surface.config.format,
                target: &target,
                surface: &surface,
                pipeline: &self.forward,
                post_process: &self.post_process,
                device: &self.device,
            };

            for node in &self.graph.nodes {
                node.render(&mut ctx);
            }

            self.queue.submit(std::iter::once(encoder.finish()));
            output.present();
        }
    }
}
