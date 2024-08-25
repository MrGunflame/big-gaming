pub mod aabb;
pub mod buffer;
pub mod camera;
pub mod entities;
pub mod forward;
pub mod graph;
pub mod light;
pub mod mesh;
pub mod metrics;
pub mod mipmap;
pub mod options;
pub mod pbr;
pub mod render_pass;
pub mod shape;
pub mod surface;
pub mod texture;

mod debug;
mod depth_stencil;
mod fps_limiter;
mod pipelined_rendering;
mod post_process;
mod state;

pub use fps_limiter::FpsLimit;
use options::MainPassOptions;

use std::collections::{HashMap, VecDeque};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use futures_lite::FutureExt;
use game_tasks::TaskPool;
use game_tracing::trace_span;

use camera::RenderTarget;
use entities::SceneEntities;
use forward::ForwardPipeline;
use game_window::windows::{WindowId, WindowState};
use glam::UVec2;
use graph::Node;
use parking_lot::Mutex;
use pbr::material::Materials;
use pbr::mesh::Meshes;
use pipelined_rendering::{Pipeline, RenderImageGpu};
use post_process::PostProcessPipeline;
use render_pass::RenderPass;
use state::RenderState;
use texture::{Images, RenderImageId, RenderTextureEvent, RenderTextures};
use thiserror::Error;
use tokio::sync::oneshot;
use wgpu::{
    Backends, Device, DeviceDescriptor, Features, Gles3MinorVersion, Instance, InstanceDescriptor,
    InstanceFlags, Limits, PowerPreference, Queue, RequestAdapterOptions, RequestDeviceError,
};

#[derive(Debug, Error)]
pub enum Error {
    #[error("no adapter")]
    NoAdapter,
    #[error("failed to request device: {}", 0)]
    NoDevice(RequestDeviceError),
}

pub struct Renderer {
    pipeline: Pipeline,

    pub entities: SceneEntities,

    backlog: VecDeque<SurfaceEvent>,
    state: Arc<Mutex<RenderState>>,

    pub images: Images,
    pub meshes: Meshes,
    pub materials: Materials,

    pub render_textures: RenderTextures,
    jobs: VecDeque<Job>,

    options: Arc<Mutex<MainPassOptions>>,
}

impl Renderer {
    pub fn new() -> Result<Self, Error> {
        let flags = if debug::debug_layers_enabled() {
            InstanceFlags::DEBUG | InstanceFlags::VALIDATION
        } else {
            InstanceFlags::empty()
        };

        let instance = Instance::new(InstanceDescriptor {
            backends: Backends::VULKAN,
            dx12_shader_compiler: Default::default(),
            flags,
            gles_minor_version: Gles3MinorVersion::Automatic,
        });

        let adapter =
            futures_lite::future::block_on(instance.request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            }))
            .ok_or(Error::NoAdapter)?;

        let features = Features::TEXTURE_BINDING_ARRAY
            | Features::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING
            | Features::PARTIALLY_BOUND_BINDING_ARRAY;

        let mut limits = Limits::default();
        limits.max_sampled_textures_per_shader_stage = 2048;

        let (device, queue) = futures_lite::future::block_on(adapter.request_device(
            &DeviceDescriptor {
                required_features: features,
                required_limits: limits,
                label: None,
            },
            None,
        ))
        .map_err(Error::NoDevice)?;

        let mut images = Images::new();
        let forward = Arc::new(ForwardPipeline::new(&device, &mut images));
        let post_process = PostProcessPipeline::new(&device);

        let state = Arc::new(Mutex::new(RenderState::new(&device, &forward, &images)));

        let pipeline = Pipeline::new(instance, adapter, device, queue);
        let options = Arc::new(Mutex::new(MainPassOptions::default()));

        {
            let mut graph = unsafe { pipeline.shared.graph.get_mut() };
            graph.push(RenderPass {
                state: state.clone(),
                forward: forward.clone(),
                post_process,
                depth_stencils: Mutex::new(HashMap::new()),
                options: options.clone(),
            });
        }

        Ok(Self {
            entities: SceneEntities::new(),
            images,
            materials: Materials::new(),
            meshes: Meshes::new(),
            backlog: VecDeque::new(),
            pipeline,
            state,
            render_textures: RenderTextures::new(),
            jobs: VecDeque::new(),
            options,
        })
    }

    pub fn read_gpu_texture(&mut self, id: RenderImageId) -> ReadTexture {
        let (tx, rx) = oneshot::channel();
        self.jobs.push_back(Job::TextureToBuffer(id, tx));
        ReadTexture { rx }
    }

    pub fn device(&self) -> &Device {
        &self.pipeline.shared.device
    }

    pub fn queue(&self) -> &Queue {
        &self.pipeline.shared.queue
    }

    pub fn add_to_graph(&self, node: impl Node) {
        self.pipeline.wait_idle();

        let mut graph = unsafe { self.pipeline.shared.graph.get_mut() };
        graph.push(node);
    }

    /// Create a new renderer for the window.
    pub fn create(&mut self, id: WindowId, window: WindowState) {
        self.backlog.push_back(SurfaceEvent::Create(id, window));
    }

    pub fn resize(&mut self, id: WindowId, size: UVec2) {
        self.backlog.push_back(SurfaceEvent::Resize(id, size));

        self.entities.cameras.for_each_mut(|_, mut camera| {
            if camera.target == RenderTarget::Window(id) {
                camera.update_aspect_ratio(size);
            }
        });
    }

    pub fn destroy(&mut self, id: WindowId) {
        self.backlog.push_back(SurfaceEvent::Destroy(id));
    }

    // TODO: Get rid of this shit.
    pub fn get_surface_size(&self, id: WindowId) -> Option<UVec2> {
        let surfaces = unsafe { self.pipeline.shared.surfaces.get() };
        surfaces
            .get(id)
            .map(|s| UVec2::new(s.config.width, s.config.height))
    }

    /// Waits until a new frame can be queued.
    pub fn wait_until_ready(&mut self) {
        let _span = trace_span!("Renderer::wait_until_ready").entered();
        self.pipeline.wait_idle();
    }

    pub fn render(&mut self, pool: &TaskPool) {
        let _span = trace_span!("Renderer::render").entered();

        self.pipeline.wait_idle();

        unsafe {
            self.update_surfaces();
        }

        {
            let mut state = self.state.lock();

            // FIXME: We should attempt to merge all event queues into a single one.
            for event in self.entities.cameras.drain_events() {
                state.update(event, &self.meshes, &self.materials, &self.images);
            }

            for event in self.entities.objects.drain_events() {
                state.update(event, &self.meshes, &self.materials, &self.images);
            }

            for event in self.entities.directional_lights.drain_events() {
                state.update(event, &self.meshes, &self.materials, &self.images);
            }

            for event in self.entities.point_lights.drain_events() {
                state.update(event, &self.meshes, &self.materials, &self.images);
            }

            for event in self.entities.spot_lights.drain_events() {
                state.update(event, &self.meshes, &self.materials, &self.images);
            }
        }

        {
            let mut render_textures = unsafe { self.pipeline.shared.render_textures.get_mut() };

            for event in self.render_textures.events.drain(..) {
                match event {
                    RenderTextureEvent::Create(id, texture) => {
                        render_textures.insert(
                            id,
                            RenderImageGpu {
                                size: texture.size,
                                texture: None,
                            },
                        );
                    }
                    RenderTextureEvent::Destroy(id) => {
                        render_textures.remove(&id);
                    }
                }
            }
        }

        {
            let mut jobs = unsafe { self.pipeline.shared.jobs.get_mut() };
            std::mem::swap(&mut self.jobs, &mut jobs);
        }

        // SAFETY: We just waited for the renderer to be idle.
        unsafe {
            self.pipeline.render_unchecked();
        }
    }

    /// # Safety
    ///
    ///  Caller guarantees that the renderer is idle.
    unsafe fn update_surfaces(&mut self) {
        let mut surfaces = unsafe { self.pipeline.shared.surfaces.get_mut() };
        let instance = &self.pipeline.shared.instance;
        let adapter = &self.pipeline.shared.adapter;
        let device = &self.pipeline.shared.device;

        while let Some(event) = self.backlog.pop_front() {
            match event {
                SurfaceEvent::Create(id, state) => {
                    surfaces.create(instance, adapter, device, state, id);
                }
                SurfaceEvent::Resize(id, size) => {
                    surfaces.resize(id, device, size);
                }
                SurfaceEvent::Destroy(id) => {
                    surfaces.destroy(id);
                }
            }
        }
    }

    pub fn set_fps_limit(&mut self, limit: FpsLimit) {
        self.jobs.push_back(Job::SetFpsLimit(limit));
    }

    pub fn set_options(&mut self, options: MainPassOptions) {
        *self.options.lock() = options;
    }
}

#[derive(Debug)]
enum SurfaceEvent {
    Create(WindowId, WindowState),
    Resize(WindowId, UVec2),
    Destroy(WindowId),
}

#[derive(Debug)]
enum Job {
    TextureToBuffer(RenderImageId, tokio::sync::oneshot::Sender<Vec<u8>>),
    SetFpsLimit(FpsLimit),
}

pub struct ReadTexture {
    rx: oneshot::Receiver<Vec<u8>>,
}

impl Future for ReadTexture {
    type Output = Vec<u8>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.get_mut().rx.poll(cx).map(|res| res.unwrap())
    }
}
