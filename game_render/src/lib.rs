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
use graph::{Node, RenderGraph};
use mesh::Mesh;
use parking_lot::Mutex;
use pbr::material::Materials;
use pbr::mesh::Meshes;
use pbr::PbrMaterial;
use pipelined_rendering::Pipeline;
use post_process::PostProcessPipeline;
use render_pass::RenderPass;
use state::RenderState;
use texture::Images;
use tracing::Instrument;
use wgpu::{
    Backends, Device, DeviceDescriptor, Features, Instance, InstanceDescriptor, Limits,
    PowerPreference, QuerySet, Queue, RequestAdapterOptions,
};

pub struct Renderer {
    pipeline: Pipeline,

    pub entities: SceneEntities,
    forward: Arc<ForwardPipeline>,

    backlog: VecDeque<Event>,
    state: Arc<Mutex<RenderState>>,

    pub images: Images,
    pub meshes: Meshes,
    pub materials: Materials,
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

        let state = Arc::new(Mutex::new(RenderState::empty(&device)));

        let pipeline = Pipeline::new(instance, adapter, device, queue);

        {
            let mut graph = pipeline.shared.graph.lock();
            graph.push(RenderPass {
                state: state.clone(),
                forward: forward.clone(),
                post_process,
            });
        }

        Self {
            entities: SceneEntities::new(),
            forward,
            images,
            materials: Materials::new(),
            meshes: Meshes::new(),
            backlog: VecDeque::new(),
            pipeline,
            state,
        }
    }

    pub fn device(&self) -> &Device {
        &self.pipeline.shared.device
    }

    pub fn queue(&self) -> &Queue {
        &self.pipeline.shared.queue
    }

    pub fn add_to_graph(&self, node: impl Node) {
        let mut graph = self.pipeline.shared.graph.lock();
        graph.push(node);
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

    // TODO: Get rid of this shit.
    pub fn get_surface_size(&self, id: WindowId) -> Option<UVec2> {
        let surfaces = self.pipeline.shared.surfaces.lock();
        surfaces
            .get(id)
            .map(|s| UVec2::new(s.config.width, s.config.height))
    }

    pub fn render(&mut self) {
        let _span = trace_span!("Renderer::render").entered();

        // FIXME: Should update on render pass.
        crate::texture::image::load_images(&mut self.images);

        self.pipeline.wait_idle();

        {
            let mut surfaces = self.pipeline.shared.surfaces.lock();
            let instance = &self.pipeline.shared.instance;
            let adapter = &self.pipeline.shared.adapter;
            let device = &self.pipeline.shared.device;
            let queue = &self.pipeline.shared.queue;
            let mut mipmap_generator = self.pipeline.shared.mipmap_generator.lock();

            while let Some(event) = self.backlog.pop_front() {
                match event {
                    Event::CreateSurface(id, state) => {
                        surfaces.create(instance, adapter, device, state, id);
                    }
                    Event::ResizeSurface(id, size) => {
                        surfaces.resize(id, device, size);
                    }
                    Event::DestroySurface(id) => {
                        surfaces.destroy(id);
                    }
                }
            }

            let state = RenderState::new(
                device,
                queue,
                &self.meshes,
                &self.materials,
                &self.images,
                &self.entities,
                &self.forward,
                &mut mipmap_generator,
            );

            *self.state.lock() = state;
        }

        // SAFETY: We just waited for the renderer to be idle.
        unsafe {
            self.pipeline.render_unchecked();
        }
    }
}

#[derive(Debug)]
enum Event {
    CreateSurface(WindowId, WindowState),
    ResizeSurface(WindowId, UVec2),
    DestroySurface(WindowId),
}
