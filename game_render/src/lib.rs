pub mod aabb;
pub mod api;
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

pub mod backend;
mod debug;
mod fps_limiter;
mod passes;
mod pipeline_cache;
mod pipelined_rendering;

use api::CommandQueue;
use backend::vulkan::{Config, Device, Instance, Queue};
use backend::{AdapterKind, QueueCapabilities};
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
use pipelined_rendering::RenderThreadHandle;
use texture::{RenderImageId, RenderTexture, RenderTextureEvent, RenderTextures};
use thiserror::Error;
use tokio::sync::oneshot;

pub use passes::FINAL_RENDER_PASS;

#[derive(Debug, Error)]
pub enum Error {
    #[error("no adapter")]
    NoAdapter,
    // #[error("failed to request device: {}", 0)]
    // NoDevice(RequestDeviceError),
}

pub struct Renderer {
    render_thread: RenderThreadHandle,

    backlog: VecDeque<SurfaceEvent>,

    forward: Arc<ForwardPipeline>,
    resources: Arc<Resources>,
    events: Vec<Event>,

    render_textures: RenderTextures,
    jobs: VecDeque<Job>,
}

impl Renderer {
    pub fn new() -> Result<Self, Error> {
        let mut config = Config::default();
        if debug::debug_layers_enabled() {
            config.validation = true;
        }

        let instance = Instance::new(config).unwrap();

        let adapter = instance
            .adapters()
            .into_iter()
            .nth(0)
            .ok_or(Error::NoAdapter)?;

        let queue_family = *adapter
            .queue_families()
            .iter()
            .find(|q| q.capabilities.contains(QueueCapabilities::GRAPHICS))
            .unwrap();

        let device = adapter.create_device(queue_family.id);
        let queue = device.queue();

        let render_thread = RenderThreadHandle::new(instance, adapter, device, queue);

        let mut scheduler = unsafe { render_thread.shared.scheduler.borrow_mut() };
        let mut graph = unsafe { render_thread.shared.graph.borrow_mut() };
        let mut queue = scheduler.queue();

        let resources = Arc::new(Resources::default());

        let forward = Arc::new(ForwardPipeline::new(&mut queue, resources.clone()));
        passes::init(&mut graph, forward.clone(), &mut queue);

        drop(graph);
        drop(scheduler);

        Ok(Self {
            render_thread,
            backlog: VecDeque::new(),
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

    pub fn with_command_queue_and_graph<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut RenderGraph, &mut CommandQueue<'_>) -> R,
    {
        self.render_thread.wait_idle();
        // SAFETY: We have waited until the render thread is idle.
        // This means we are allowed to access all shared resources.
        let mut scheduler = unsafe { self.render_thread.shared.scheduler.borrow_mut() };
        let mut graph = unsafe { self.render_thread.shared.graph.borrow_mut() };
        let mut queue = scheduler.queue();
        f(&mut graph, &mut queue)
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
                let surfaces = unsafe { self.render_thread.shared.surfaces.borrow() };
                surfaces.get(id).map(|s| s.config.extent)
            }
            RenderTarget::Image(id) => self.render_textures.get(id).map(|v| v.size),
        }
    }

    /// Waits until a new frame can be queued.
    pub fn wait_until_ready(&mut self) {
        let _span = trace_span!("Renderer::wait_until_ready").entered();
        self.render_thread.wait_idle();
    }

    pub fn render(&mut self, pool: &TaskPool) {
        let _span = trace_span!("Renderer::render").entered();

        self.render_thread.wait_idle();

        unsafe {
            self.update_surfaces();

            // Commit all new resources.
            // This is safe since the renderer is idle.
            self.resources.commit();
            core::mem::swap(&mut *self.forward.events.borrow_mut(), &mut self.events);
            self.events.clear();
        }

        // {
        //     let mut render_textures = unsafe { self.pipeline.shared.render_textures.borrow_mut() };

        //     for event in self.render_textures.events.drain(..) {
        //         match event {
        //             RenderTextureEvent::Create(id, texture) => {
        //                 render_textures.insert(
        //                     id,
        //                     RenderImageGpu {
        //                         size: texture.size,
        //                         texture: None,
        //                     },
        //                 );
        //             }
        //             RenderTextureEvent::Destroy(id) => {
        //                 render_textures.remove(&id);
        //             }
        //         }
        //     }
        // }

        // {
        //     let mut jobs = unsafe { self.pipeline.shared.jobs.borrow_mut() };
        //     std::mem::swap(&mut self.jobs, &mut jobs);
        // }

        // SAFETY: We just waited for the renderer to be idle.
        unsafe {
            self.render_thread.render_unchecked();
        }
    }

    /// # Safety
    ///
    ///  Caller guarantees that the renderer is idle.
    unsafe fn update_surfaces(&mut self) {
        let mut surfaces = unsafe { self.render_thread.shared.surfaces.borrow_mut() };
        let instance = &self.render_thread.shared.instance;
        let adapter = &self.render_thread.shared.adapter;
        let device = &self.render_thread.shared.device;

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
        self.render_thread.shutdown();
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
