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
pub mod pbr;
pub mod render_pass;
pub mod shape;
pub mod surface;
pub mod texture;

mod depth_stencil;
mod pipelined_rendering;
mod post_process;
mod state;

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
use texture::image::ImageLoader;
use texture::{Images, RenderImageId, RenderTextureEvent, RenderTextures};
use tokio::sync::oneshot;
use wgpu::{
    Backends, Device, DeviceDescriptor, Features, Instance, InstanceDescriptor, Limits,
    PowerPreference, Queue, RequestAdapterOptions,
};

pub struct Renderer {
    pipeline: Pipeline,

    pub entities: SceneEntities,

    backlog: VecDeque<SurfaceEvent>,
    state: Arc<Mutex<RenderState>>,

    pub images: Images,
    pub meshes: Meshes,
    pub materials: Materials,

    image_loader: ImageLoader,
    pub render_textures: RenderTextures,
    jobs: VecDeque<Job>,
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

        let state = Arc::new(Mutex::new(RenderState::new(&device, &forward, &images)));

        let pipeline = Pipeline::new(instance, adapter, device, queue);

        {
            let mut graph = unsafe { pipeline.shared.graph.get_mut() };
            graph.push(RenderPass {
                state: state.clone(),
                forward: forward.clone(),
                post_process,
                depth_stencils: Mutex::new(HashMap::new()),
            });
        }

        Self {
            entities: SceneEntities::new(),
            images,
            materials: Materials::new(),
            meshes: Meshes::new(),
            backlog: VecDeque::new(),
            pipeline,
            state,
            image_loader: ImageLoader::default(),
            render_textures: RenderTextures::new(),
            jobs: VecDeque::new(),
        }
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

    pub fn render(&mut self, pool: &TaskPool) {
        let _span = trace_span!("Renderer::render").entered();

        self.image_loader.update(&mut self.images, pool);

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
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
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
