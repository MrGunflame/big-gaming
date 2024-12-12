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
pub mod shape;
pub mod surface;
pub mod texture;

mod debug;
mod depth_stencil;
mod fps_limiter;
mod passes;
mod pipeline_cache;
mod pipelined_rendering;

use entities::{Event, Resources, ResourcesMut};
pub use fps_limiter::FpsLimit;
use game_common::cell::RefMut;

use std::collections::VecDeque;
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
use pipelined_rendering::{Pipeline, RenderImageGpu};
use texture::{RenderImageId, RenderTexture, RenderTextureEvent, RenderTextures};
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

    forward: Arc<ForwardPipeline>,
    resources: Arc<Resources>,
    events: Vec<Event>,

    render_textures: RenderTextures,
    jobs: VecDeque<Job>,
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

        let resources = Arc::new(Resources::default());

        let forward = Arc::new(ForwardPipeline::new(&device, resources.clone()));

        let pipeline = Pipeline::new(instance, adapter, device, queue);

        {
            let mut graph = unsafe { pipeline.shared.graph.borrow_mut() };
            passes::init(
                &mut graph,
                forward.clone(),
                &pipeline.shared.device,
                &pipeline.shared.queue,
            );
        }

        Ok(Self {
            backlog: VecDeque::new(),
            pipeline,
            render_textures: RenderTextures::new(),
            jobs: VecDeque::new(),
            forward,
            resources,
            events: Vec::new(),
        })
    }

    pub fn resources(&mut self) -> ResourcesMut<'_> {
        unsafe { ResourcesMut::new(&self.resources, &mut self.events) }
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
        id
    }

    pub fn destory_render_texture(&mut self, id: RenderImageId) {}

    /// Create a new renderer for the window.
    pub fn create(&mut self, id: WindowId, window: WindowState) {
        self.backlog.push_back(SurfaceEvent::Create(id, window));
    }

    pub fn resize(&mut self, id: WindowId, size: UVec2) {
        self.backlog.push_back(SurfaceEvent::Resize(id, size));
    }

    pub fn destroy(&mut self, id: WindowId) {
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

            // Commit all new resources.
            // This is safe since the renderer is idle.
            self.resources.commit();
            core::mem::swap(&mut *self.forward.events.borrow_mut(), &mut self.events);
            self.events.clear();
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

                    // Resize all cameras that are linked to the surface handle.
                    let mut cameras = unsafe { self.resources.cameras.viewer() };
                    for camera in cameras.iter_mut() {
                        if camera.target == RenderTarget::Window(id) {
                            camera.update_aspect_ratio(size);
                        }
                    }
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
