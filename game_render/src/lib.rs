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
pub mod pipeline_cache;
pub mod shader;
pub mod shape;
pub mod statistics;
pub mod surface;
pub mod texture;

pub mod backend;
mod debug;
mod fps_limiter;
mod passes;
mod pipelined_rendering;

use api::CommandQueue;
use backend::vulkan::{Config, Instance};
use backend::{AdapterKind, MemoryHeapFlags, QueueCapabilities};
use entities::{Event, Resources, ResourcesMut};
pub use fps_limiter::FpsLimit;
use game_tasks::park::Parker;
use statistics::Statistics;
use surface::SurfaceConfig;

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
use pipelined_rendering::{Command, RenderThreadHandle};
use texture::{RenderImageId, RenderTexture, RenderTextures};
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

    forward: Arc<ForwardPipeline>,
    resources: Arc<Resources>,
    events: Vec<Event>,

    render_textures: RenderTextures,
    jobs: VecDeque<Job>,

    parker: Option<Arc<Parker>>,

    surface_sizes: HashMap<WindowId, UVec2>,

    statistics: Arc<Statistics>,
}

impl Renderer {
    pub fn new() -> Result<Self, Error> {
        let mut config = Config::default();
        config.validation = debug::debug_enabled();
        config.gpuav = debug::gpuav_enabled();

        let instance = Instance::new(config).unwrap();

        let preferred_adapter_id = std::env::var("RENDER_ADAPTER_ID")
            .map(|v| v.parse::<usize>().ok())
            .ok()
            .flatten();
        let preferred_adapter_name = std::env::var("RENDER_ADAPTER").ok();

        let mut adapter = None;
        for (adapter_id, a) in instance.adapters().unwrap().into_iter().enumerate() {
            if Some(adapter_id) == preferred_adapter_id {
                adapter = Some(a);
                break;
            }

            let Some(current_adapter) = &adapter else {
                adapter = Some(a);
                continue;
            };

            if let Some(preferred_adapter_name) = &preferred_adapter_name {
                if a.properties().name.contains(preferred_adapter_name) {
                    adapter = Some(a);
                    continue;
                }
            }

            let new_kind = match a.properties().kind {
                AdapterKind::DiscreteGpu => 0,
                AdapterKind::IntegratedGpu => 1,
                AdapterKind::Cpu => 2,
                AdapterKind::Other => 3,
            };
            let cur_kind = match current_adapter.properties().kind {
                AdapterKind::DiscreteGpu => 0,
                AdapterKind::IntegratedGpu => 1,
                AdapterKind::Cpu => 2,
                AdapterKind::Other => 3,
            };

            // New adapter is a less powerful class than the current one.
            // Alaways ignore the new adapter.
            if new_kind > cur_kind {
                continue;
            }

            // New adapter is a more powerful class than the current one.
            // Always prefer the new adapter.
            if new_kind < cur_kind {
                adapter = Some(a);
                continue;
            }

            // Both adapters have the same class.
            // Choose the adapter with more device local memory, which
            // is usually the more powerful one.
            let mut new_device_local = 0;
            for heap in a.memory_properties().heaps {
                if heap.flags.contains(MemoryHeapFlags::DEVICE_LOCAL) {
                    new_device_local += heap.size;
                }
            }

            let mut cur_device_local = 0;
            for heap in current_adapter.memory_properties().heaps {
                if heap.flags.contains(MemoryHeapFlags::DEVICE_LOCAL) {
                    cur_device_local += heap.size;
                }
            }

            if new_device_local > cur_device_local {
                adapter = Some(a);
            }
        }

        let Some(adapter) = adapter else {
            return Err(Error::NoAdapter);
        };

        tracing::info!("Using graphics adapter {}", adapter.properties().name);

        let queue_family = *adapter
            .queue_families()
            .iter()
            .find(|q| {
                q.capabilities.contains(QueueCapabilities::GRAPHICS)
                    && q.capabilities.contains(QueueCapabilities::TRANSFER)
            })
            .unwrap();

        let mut device = adapter.create_device(&[queue_family]).unwrap();
        let queue = device.create_queue(queue_family.id).unwrap();

        let statistics = Arc::new(Statistics::default());
        let render_thread =
            RenderThreadHandle::new(instance, adapter, device, queue, statistics.clone());

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
            render_textures: RenderTextures::new(),
            jobs: VecDeque::new(),
            forward,
            resources,
            events: Vec::new(),
            surface_sizes: HashMap::new(),
            parker: None,
            statistics,
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
        F: FnOnce(&mut RenderGraph, &CommandQueue<'_>) -> R,
    {
        if let Some(parker) = self.parker.take() {
            parker.park();
        }

        // SAFETY: We have waited until the render thread is idle.
        // This means we are allowed to access all shared resources.
        let scheduler = unsafe { self.render_thread.shared.scheduler.borrow_mut() };
        let mut graph = unsafe { self.render_thread.shared.graph.borrow_mut() };
        let queue = scheduler.queue();
        f(&mut graph, &queue)
    }

    pub fn create_render_texture(&mut self, texture: RenderTexture) -> RenderImageId {
        let id = self.render_textures.insert(texture);
        id
    }

    pub fn destory_render_texture(&mut self, id: RenderImageId) {}

    /// Create a new renderer for the window.
    pub fn create(&mut self, id: WindowId, window: WindowState) {
        self.surface_sizes.insert(id, window.inner_size());
        self.render_thread.send(Command::CreateSurface(window));
    }

    pub fn resize(&mut self, id: WindowId, size: UVec2) {
        self.surface_sizes.insert(id, size);
        self.render_thread
            .send(Command::UpdateSurface(id, SurfaceConfig { size }));

        if let Some(parker) = self.parker.take() {
            parker.park();
        }

        // Resize all cameras that are linked to the surface handle.
        // Technically this will happend before the actual surface is resized,
        // but this should be considered a rare case anyway, so we can afford
        // the aspect ratio to be slightly off during that time.
        let mut cameras = unsafe { self.resources.cameras.viewer() };
        for camera in cameras.iter_mut() {
            if camera.target == RenderTarget::Window(id) {
                camera.update_aspect_ratio(size);
            }
        }
    }

    pub fn destroy(&mut self, id: WindowId) {
        self.render_thread.send(Command::DestroySurface(id));
    }

    // TODO: Get rid of this shit.
    pub fn get_surface_size(&self, target: RenderTarget) -> Option<UVec2> {
        match target {
            RenderTarget::Window(id) => self.surface_sizes.get(&id).copied(),
            RenderTarget::Image(id) => {
                todo!();
            }
        }
    }

    /// Waits until a new frame can be queued.
    pub fn wait_until_ready(&mut self) {
        let _span = trace_span!("Renderer::wait_until_ready").entered();

        if let Some(parker) = self.parker.take() {
            parker.park();
        }
    }

    pub fn render(&mut self, pool: &TaskPool) {
        let _span = trace_span!("Renderer::render").entered();

        // Park until the previous rendering command has finished.
        // Once the `park()` call completes the renderer will no longer
        // access its shared resources.
        // If the parker is `None`, no rendering command was submitted yet,
        // so the renderer is also idle.
        if let Some(parker) = &self.parker {
            parker.park();
        }

        unsafe {
            // Commit all new resources.
            // This is safe since the renderer is idle.
            self.resources.commit();
            core::mem::swap(&mut *self.forward.events.borrow_mut(), &mut self.events);
            self.events.clear();
        }

        // Submit a new render command with the parker used in the next call.
        let parker = self.parker.get_or_insert_with(|| Arc::new(Parker::new()));
        self.render_thread.send(Command::Render(parker.clone()));
    }

    pub fn set_fps_limit(&mut self, limit: FpsLimit) {
        for id in self.surface_sizes.keys() {
            self.render_thread
                .send(Command::UpdateSurfaceFpsLimit(*id, limit));
        }
    }

    pub fn statistics(&self) -> &Arc<Statistics> {
        &self.statistics
    }
}

#[derive(Debug)]
enum Job {
    TextureToBuffer(RenderImageId, tokio::sync::oneshot::Sender<Vec<u8>>),
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
