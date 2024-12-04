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
pub mod scene;
pub mod shape;
pub mod surface;
pub mod texture;

mod debug;
mod depth_stencil;
mod fps_limiter;
mod passes;
mod pipeline_cache;
mod pipelined_rendering;
mod state;

pub use fps_limiter::FpsLimit;
use game_common::cell::RefMut;
use scene::{RendererScene, Scene};

use std::collections::{HashMap, VecDeque};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use futures_lite::FutureExt;
use game_tasks::TaskPool;
use game_tracing::trace_span;

use camera::RenderTarget;
use forward::ForwardPipeline;
use game_window::windows::{WindowId, WindowState};
use glam::UVec2;
use graph::RenderGraph;
use parking_lot::Mutex;
use pbr::material::Materials;
use pbr::mesh::Meshes;
use pipelined_rendering::{Pipeline, RenderImageGpu};
use state::RenderState;
use texture::{Images, RenderImageId, RenderTexture, RenderTextureEvent, RenderTextures};
use thiserror::Error;
use tokio::sync::oneshot;
use wgpu::{
    Backends, Device, DeviceDescriptor, Features, Gles3MinorVersion, Instance, InstanceDescriptor,
    InstanceFlags, Limits, PowerPreference, Queue, RequestAdapterOptions, RequestDeviceError,
};

pub use passes::FINAL_RENDER_PASS;

#[derive(Debug, Error)]
pub enum Error {
    #[error("no adapter")]
    NoAdapter,
    #[error("failed to request device: {}", 0)]
    NoDevice(RequestDeviceError),
}

pub struct Renderer {
    pipeline: Pipeline,

    backlog: VecDeque<SurfaceEvent>,

    pub images: Images,
    pub meshes: Meshes,
    pub materials: Materials,

    render_textures: RenderTextures,
    jobs: VecDeque<Job>,

    scenes: HashMap<RenderTarget, Scene>,
    state: Arc<Mutex<HashMap<RenderTarget, RenderState>>>,
    forward: Arc<ForwardPipeline>,
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
            | Features::PARTIALLY_BOUND_BINDING_ARRAY
            | Features::PUSH_CONSTANTS;

        let mut limits = Limits::default();
        limits.max_sampled_textures_per_shader_stage = 2048;
        limits.max_push_constant_size = 128;

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

        let state = Arc::new(Mutex::new(HashMap::new()));

        let pipeline = Pipeline::new(instance, adapter, device, queue);

        {
            let mut graph = unsafe { pipeline.shared.graph.borrow_mut() };
            passes::init(
                &mut graph,
                state.clone(),
                forward.clone(),
                &pipeline.shared.device,
            );
        }

        Ok(Self {
            images,
            materials: Materials::new(),
            meshes: Meshes::new(),
            backlog: VecDeque::new(),
            pipeline,
            state,
            render_textures: RenderTextures::new(),
            jobs: VecDeque::new(),
            scenes: HashMap::new(),
            forward,
        })
    }

    pub fn scene_mut(&mut self, target: RenderTarget) -> Option<RendererScene<'_>> {
        let size = self.get_surface_size(target)?;

        self.scenes.get_mut(&target).map(|scene| RendererScene {
            scene,
            meshes: &mut self.meshes,
            images: &mut self.images,
            materials: &mut self.materials,
            size,
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

    pub fn graph_mut(&mut self) -> RefMut<'_, RenderGraph> {
        self.pipeline.wait_idle();
        unsafe { self.pipeline.shared.graph.borrow_mut() }
    }

    pub fn create_render_texture(&mut self, texture: RenderTexture) -> RenderImageId {
        let id = self.render_textures.insert(texture);
        self.scenes.insert(id.into(), Scene::default());
        // FIXME: Bad because it blocks until the next render pass.
        self.state.lock().insert(
            id.into(),
            RenderState::new(&self.pipeline.shared.device, &self.forward, &self.images),
        );
        id
    }

    pub fn destory_render_texture(&mut self, id: RenderImageId) {
        self.scenes.remove(&id.into());
        // FIXME: Bad because it blocks until the next render pass.
        self.state.lock().remove(&id.into());
    }

    /// Create a new renderer for the window.
    pub fn create(&mut self, id: WindowId, window: WindowState) {
        self.scenes.insert(id.into(), Scene::default());
        // FIXME: Bad because it blocks until the next render pass.
        self.state.lock().insert(
            id.into(),
            RenderState::new(&self.pipeline.shared.device, &self.forward, &self.images),
        );
        self.backlog.push_back(SurfaceEvent::Create(id, window));
    }

    pub fn resize(&mut self, id: WindowId, size: UVec2) {
        self.backlog.push_back(SurfaceEvent::Resize(id, size));

        for scene in self.scenes.values_mut() {
            scene.entities.cameras.for_each_mut(|_, mut camera| {
                if camera.target == RenderTarget::Window(id) {
                    camera.update_aspect_ratio(size);
                }
            });
        }
    }

    pub fn destroy(&mut self, id: WindowId) {
        self.scenes.remove(&id.into());
        // FIXME: Bad because it blocks until the next render pass.
        self.state.lock().remove(&id.into());
        self.backlog.push_back(SurfaceEvent::Destroy(id));
    }

    // TODO: Get rid of this shit.
    pub fn get_surface_size(&self, target: RenderTarget) -> Option<UVec2> {
        match target {
            RenderTarget::Window(id) => {
                let surfaces = unsafe { self.pipeline.shared.surfaces.borrow() };
                surfaces
                    .get(id)
                    .map(|s| UVec2::new(s.config.width, s.config.height))
            }
            RenderTarget::Image(id) => self.render_textures.get(id).map(|v| v.size),
        }
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

        for (target, scene) in self.scenes.iter_mut() {
            let mut state = self.state.lock();
            let state = state.get_mut(target).unwrap();

            // FIXME: We should attempt to merge all event queues into a single one.
            for event in scene.entities.cameras.drain_events() {
                state.update(event, &self.meshes, &self.materials, &self.images);
            }

            for event in scene.entities.objects.drain_events() {
                state.update(event, &self.meshes, &self.materials, &self.images);
            }

            for event in scene.entities.directional_lights.drain_events() {
                state.update(event, &self.meshes, &self.materials, &self.images);
            }

            for event in scene.entities.point_lights.drain_events() {
                state.update(event, &self.meshes, &self.materials, &self.images);
            }

            for event in scene.entities.spot_lights.drain_events() {
                state.update(event, &self.meshes, &self.materials, &self.images);
            }

            for event in scene.events.drain(..) {
                state.update(event, &self.meshes, &self.materials, &self.images);
            }
        }

        {
            let mut render_textures = unsafe { self.pipeline.shared.render_textures.borrow_mut() };

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
            let mut jobs = unsafe { self.pipeline.shared.jobs.borrow_mut() };
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
        let mut surfaces = unsafe { self.pipeline.shared.surfaces.borrow_mut() };
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
}

impl Drop for Renderer {
    fn drop(&mut self) {
        self.pipeline.shutdown();
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
