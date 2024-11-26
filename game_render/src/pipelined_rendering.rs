use std::collections::{HashMap, VecDeque};
use std::marker::PhantomData;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::Arc;

use game_common::cell::UnsafeRefCell;
use game_tasks::park::Parker;
use game_tracing::trace_span;
use glam::UVec2;
use wgpu::hal::auxil::db;
use wgpu::{
    Adapter, BufferAddress, BufferDescriptor, BufferUsages, CommandEncoderDescriptor, Device,
    Extent3d, ImageCopyBuffer, ImageCopyTexture, ImageDataLayout, Instance, MapMode, Origin3d,
    Queue, Texture, TextureAspect, TextureDescriptor, TextureDimension, TextureFormat,
    TextureUsages, TextureViewDescriptor, COPY_BYTES_PER_ROW_ALIGNMENT,
};

use crate::camera::RenderTarget;
use crate::fps_limiter::{FpsLimit, FpsLimiter};
use crate::graph::scheduler::RenderGraphScheduler;
use crate::graph::{RenderContext, RenderGraph, SlotLabel, SlotValueInner};
use crate::mipmap::MipMapGenerator;
use crate::surface::RenderSurfaces;
use crate::texture::RenderImageId;
use crate::Job;

const PIPELINE_STATE_RENDERING: u8 = 1;
const PIPELINE_STATE_IDLE: u8 = 2;
const PIPELINE_STATE_EXIT: u8 = 3;

pub struct SharedState {
    pub instance: Instance,
    pub adapter: Adapter,
    pub device: Device,
    pub queue: Queue,
    pub surfaces: UnsafeRefCell<RenderSurfaces>,
    pub render_textures: UnsafeRefCell<HashMap<RenderImageId, RenderImageGpu>>,
    mipmap_generator: UnsafeRefCell<MipMapGenerator>,
    pub graph: UnsafeRefCell<RenderGraph>,
    state: AtomicU8,
    /// Unparker for the calling thread.
    main_unparker: Arc<Parker>,
    pub jobs: UnsafeRefCell<VecDeque<Job>>,
    fps_limiter: UnsafeRefCell<FpsLimiter>,
    shutdown: AtomicBool,
}

pub struct Pipeline {
    pub shared: Arc<SharedState>,
    main_parker: Arc<Parker>,
    /// Unparker for the render thread.
    render_unparker: Arc<Parker>,
    // While `Pipeline` is not directly thread-unsafe, we make no guarantees
    // whether atomic operations hold up when dispatching renders from multiple
    // threads.
    _marker: PhantomData<*const ()>,
}

impl Pipeline {
    pub fn new(instance: Instance, adapter: Adapter, device: Device, queue: Queue) -> Self {
        let main_parker = Arc::new(Parker::new());
        let main_unparker = main_parker.clone();

        let shared = Arc::new(SharedState {
            mipmap_generator: UnsafeRefCell::new(MipMapGenerator::new(&device)),
            instance,
            adapter,
            device,
            queue,
            surfaces: UnsafeRefCell::new(RenderSurfaces::new()),
            state: AtomicU8::new(PIPELINE_STATE_IDLE),
            graph: UnsafeRefCell::new(RenderGraph::default()),
            main_unparker,
            render_textures: UnsafeRefCell::new(HashMap::new()),
            jobs: UnsafeRefCell::new(VecDeque::new()),
            fps_limiter: UnsafeRefCell::new(FpsLimiter::new(FpsLimit::UNLIMITED)),
            shutdown: AtomicBool::new(false),
        });

        let render_unparker = start_render_thread(shared.clone());

        Self {
            shared,
            render_unparker,
            main_parker,
            _marker: PhantomData,
        }
    }

    pub fn is_idle(&self) -> bool {
        self.shared.state.load(Ordering::Acquire) == PIPELINE_STATE_IDLE
    }

    pub fn wait_idle(&self) {
        let _span = trace_span!("Pipeline::wait_idle").entered();

        while !self.is_idle() {
            self.main_parker.park();
        }
    }

    /// # Safety
    ///
    /// renderer must be idle.
    pub unsafe fn render_unchecked(&mut self) {
        debug_assert!(self.is_idle());

        self.shared
            .state
            .store(PIPELINE_STATE_RENDERING, Ordering::Release);

        self.render_unparker.unpark();
    }

    pub fn shutdown(&mut self) {
        self.shared.shutdown.store(true, Ordering::Release);
        self.render_unparker.unpark();
    }
}

fn start_render_thread(shared: Arc<SharedState>) -> Arc<Parker> {
    let parker = Arc::new(Parker::new());
    let unparker = parker.clone();

    std::thread::spawn(move || {
        let _span = trace_span!("render_thread").entered();
        loop {
            if shared.shutdown.load(Ordering::Acquire) {
                return;
            }

            // FIXME: If it is guaranteed that the parker will never yield
            // before being signaled, there is not need to watch for the atomic
            // to change.
            while shared.state.load(Ordering::Acquire) != PIPELINE_STATE_RENDERING {
                parker.park();
            }

            // SAFETY: The pipeline is in rendering state, the render thread
            // has full access to the state.
            unsafe {
                execute_render(&shared);
            }

            shared.state.store(PIPELINE_STATE_IDLE, Ordering::Relaxed);
            shared.main_unparker.unpark();
        }
    });

    unparker
}

unsafe fn execute_render(shared: &SharedState) {
    let _span = trace_span!("render_frame").entered();

    let surfaces = unsafe { shared.surfaces.borrow() };
    let graph = unsafe { shared.graph.borrow() };
    let mut mipmap = unsafe { shared.mipmap_generator.borrow_mut() };
    let mut fps_limiter = unsafe { shared.fps_limiter.borrow_mut() };

    let mut encoder = shared
        .device
        .create_command_encoder(&CommandEncoderDescriptor { label: None });

    let mut outputs = Vec::new();

    let render_passes = RenderGraphScheduler.schedule(&graph).unwrap();
    dbg!(&render_passes);

    for (window, surface) in surfaces.iter() {
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

        let mut resources = HashMap::new();
        resources.insert(
            SlotLabel::SURFACE,
            SlotValueInner::TextureRef(&output.texture),
        );

        for node in &render_passes {
            let node = graph.get(*node).unwrap();

            let mut ctx = RenderContext {
                render_target: RenderTarget::Window(*window),
                encoder: &mut encoder,
                size: UVec2::new(surface.config.width, surface.config.height),
                target: &target,
                format: surface.config.format,
                device: &shared.device,
                queue: &shared.queue,
                mipmap: &mut mipmap,
                resources: &mut resources,
                resource_permissions: &node.permissions,
            };

            node.node.render(&mut ctx);
        }

        outputs.push((surface, output));
    }

    let mut render_textures = unsafe { shared.render_textures.borrow_mut() };
    for (id, render_texture) in render_textures.iter_mut() {
        let texture = render_texture.texture.get_or_insert_with(|| {
            let texture = shared.device.create_texture(&TextureDescriptor {
                label: None,
                size: Extent3d {
                    width: render_texture.size.x,
                    height: render_texture.size.y,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba8Unorm,
                usage: TextureUsages::COPY_SRC | TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });

            texture
        });

        let target = texture.create_view(&TextureViewDescriptor::default());

        let mut resources = HashMap::new();
        resources.insert(SlotLabel::SURFACE, SlotValueInner::TextureRef(&texture));

        for node in &render_passes {
            let node = graph.get(*node).unwrap();

            let mut ctx = RenderContext {
                render_target: RenderTarget::Image(*id),
                encoder: &mut encoder,
                size: render_texture.size,
                target: &target,
                format: texture.format(),
                device: &shared.device,
                queue: &shared.queue,
                mipmap: &mut mipmap,
                resources: &mut resources,
                resource_permissions: &node.permissions,
            };

            node.node.render(&mut ctx);
        }
    }

    let mut mapping_buffers = Vec::new();

    let mut jobs = unsafe { shared.jobs.borrow_mut() };
    for job in jobs.drain(..) {
        match job {
            Job::SetFpsLimit(limit) => {
                *fps_limiter = FpsLimiter::new(limit);
            }
            Job::TextureToBuffer(id, tx) => {
                let texture = render_textures.get(&id).unwrap();

                // bytes_per_row must be aligned as required by wgpu.
                // 4 for RGBA8
                let mut bytes_per_row = 4 * texture.size.x;
                if bytes_per_row & COPY_BYTES_PER_ROW_ALIGNMENT != 0 {
                    bytes_per_row &= u32::MAX & !COPY_BYTES_PER_ROW_ALIGNMENT;
                    bytes_per_row += COPY_BYTES_PER_ROW_ALIGNMENT;
                }

                let buffer_size = bytes_per_row * texture.size.y;

                let buffer = shared.device.create_buffer(&BufferDescriptor {
                    size: buffer_size as BufferAddress,
                    usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
                    mapped_at_creation: false,
                    label: None,
                });

                encoder.copy_texture_to_buffer(
                    ImageCopyTexture {
                        aspect: TextureAspect::All,
                        mip_level: 0,
                        origin: Origin3d::ZERO,
                        texture: texture.texture.as_ref().unwrap(),
                    },
                    ImageCopyBuffer {
                        buffer: &buffer,
                        layout: ImageDataLayout {
                            offset: 0,
                            bytes_per_row: Some(bytes_per_row),
                            rows_per_image: None,
                        },
                    },
                    Extent3d {
                        width: texture.size.x,
                        height: texture.size.y,
                        depth_or_array_layers: 1,
                    },
                );

                mapping_buffers.push((buffer, tx));
            }
        }
    }

    shared.queue.submit(std::iter::once(encoder.finish()));

    fps_limiter.block_until_ready();

    for (surface, output) in outputs {
        surface.window().pre_present_notify();
        output.present();
    }

    for (buffer, tx) in mapping_buffers {
        // Unfortunately we need to wrap `Buffer` in `Arc` to be able
        // to call `map_async` on the same value that takes a closure
        // that also moves the value.
        let buffer = Arc::new(buffer);

        buffer
            .clone()
            .slice(..)
            .map_async(MapMode::Read, move |res| {
                res.unwrap();

                {
                    let slice = buffer.slice(..);
                    let data = slice.get_mapped_range();
                    let _ = tx.send(data.to_vec());
                }

                buffer.unmap();
            });
    }
}

#[derive(Debug)]
pub(crate) struct RenderImageGpu {
    pub(crate) size: UVec2,
    /// Texture if initiliazed.
    pub(crate) texture: Option<Texture>,
}
