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
mod pipelined_rendering;
mod post_process;
mod state;

use std::collections::VecDeque;
use std::sync::Arc;

use game_tracing::trace_span;

use camera::RenderTarget;
use entities::SceneEntities;
use forward::ForwardPipeline;
use game_asset::Assets;
use game_window::windows::{WindowId, WindowState};
use glam::UVec2;
use graph::RenderGraph;
use mesh::Mesh;
use pbr::PbrMaterial;
use pipelined_rendering::Pipeline;
use post_process::PostProcessPipeline;
use render_pass::RenderPass;
use state::RenderState;
use texture::Images;
use wgpu::{
    Backends, DeviceDescriptor, Features, Instance, InstanceDescriptor, Limits, PowerPreference,
    RequestAdapterOptions,
};

pub struct Renderer {
    pipeline: Pipeline,

    pub graph: RenderGraph,
    pub entities: SceneEntities,
    pub images: Images,
    forward: Arc<ForwardPipeline>,
    pub meshes: Assets<Mesh>,
    pub materials: Assets<PbrMaterial>,
    backlog: VecDeque<Event>,
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

        let mut images = Images::new();
        let forward = Arc::new(ForwardPipeline::new(&device, &mut images));
        let post_process = PostProcessPipeline::new(&device);

        let mut graph = RenderGraph::default();
        graph.push(RenderPass {
            state: RenderState::empty(&device),
            forward: forward.clone(),
            post_process,
        });

        let pipeline = Pipeline::new(instance, adapter, device, queue);

        Self {
            entities: SceneEntities::new(),
            forward,
            graph,
            images,
            materials: Assets::new(),
            meshes: Assets::new(),
            backlog: VecDeque::new(),
            pipeline,
        }
    }

    /// Create a new renderer for the window.
    pub fn create(&mut self, id: WindowId, window: WindowState) {
        self.backlog.push_back(Event::CreateSurface(id, window));
    }

    pub fn resize(&mut self, id: WindowId, size: UVec2) {
        self.backlog.push_back(Event::ResizeSurface(id, size));

        for cam in self.entities.cameras.values_mut() {
            if cam.target == RenderTarget::Window(id) {
                cam.update_aspect_ratio(size);
            }
        }
    }

    pub fn destroy(&mut self, id: WindowId) {
        self.backlog.push_back(Event::DestroySurface(id));
    }

    pub fn render(&mut self) {
        let _span = trace_span!("Renderer::render").entered();

        // FIXME: Should update on render pass.
        crate::texture::image::load_images(&mut self.images);

        //     for (window, surface) in self.surfaces.iter_mut() {
        //         let output = match surface.surface.get_current_texture() {
        //             Ok(output) => output,
        //             Err(err) => {
        //                 tracing::error!("failed to get surface: {}", err);
        //                 continue;
        //             }
        //         };

        //         let target = output.texture.create_view(&TextureViewDescriptor {
        //             label: Some("surface_view"),
        //             format: Some(surface.config.format),
        //             ..Default::default()
        //         });

        //         let mut encoder = self
        //             .device
        //             .create_command_encoder(&CommandEncoderDescriptor {
        //                 label: Some("render_encoder"),
        //             });

        //         let mut ctx = RenderContext {
        //             window: *window,
        //             encoder: &mut encoder,
        //             width: output.texture.width(),
        //             height: output.texture.height(),
        //             format: surface.config.format,
        //             target: &target,
        //             surface: &surface,
        //             pipeline: &self.forward,
        //             post_process: &self.post_process,
        //             device: &self.device,
        //         };

        //         for node in &self.graph.nodes {
        //             node.render(&mut ctx);
        //         }

        //         self.queue.submit(std::iter::once(encoder.finish()));
        //         output.present();
        //     }
    }
}

#[derive(Debug)]
enum Event {
    CreateSurface(WindowId, WindowState),
    ResizeSurface(WindowId, UVec2),
    DestroySurface(WindowId),
}
