use std::collections::{HashMap, VecDeque};
use std::marker::PhantomData;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::Arc;
use std::time::Instant;

use ash::vk::PipelineStageFlags;
use game_common::cell::UnsafeRefCell;
use game_common::collections::scratch_buffer::ScratchBuffer;
use game_tasks::park::Parker;
use game_tracing::trace_span;

use crate::api::CommandExecutor;
use crate::backend::vulkan::{Adapter, CommandPool, Device, Instance, Queue};
use crate::backend::{AccessFlags, PipelineBarriers, QueueSubmit, TextureBarrier, TextureUsage};
use crate::camera::RenderTarget;
use crate::fps_limiter::{FpsLimit, FpsLimiter};
use crate::graph::scheduler::RenderGraphScheduler;
use crate::graph::{NodeLabel, RenderContext, RenderGraph, SlotLabel, SlotValueInner};
use crate::surface::RenderSurfaces;
use crate::Job;

const PIPELINE_STATE_RENDERING: u8 = 1;
const PIPELINE_STATE_IDLE: u8 = 2;
const PIPELINE_STATE_EXIT: u8 = 3;

pub struct SharedState {
    pub instance: Instance,
    pub adapter: Adapter,
    pub device: Device,
    pub surfaces: UnsafeRefCell<RenderSurfaces>,
    // render_textures: UnsafeRefCell<HashMap<RenderImageId, RenderImageGpu>>,
    pub graph: UnsafeRefCell<RenderGraph>,
    state: AtomicU8,
    /// Unparker for the calling thread.
    main_unparker: Arc<Parker>,
    pub jobs: UnsafeRefCell<VecDeque<Job>>,
    fps_limiter: UnsafeRefCell<FpsLimiter>,
    shutdown: AtomicBool,
    pub scheduler: UnsafeRefCell<CommandExecutor>,
}

pub struct RenderThreadHandle {
    pub shared: Arc<SharedState>,
    main_parker: Arc<Parker>,
    /// Unparker for the render thread.
    render_unparker: Arc<Parker>,
    // While `Pipeline` is not directly thread-unsafe, we make no guarantees
    // whether atomic operations hold up when dispatching renders from multiple
    // threads.
    _marker: PhantomData<*const ()>,
}

impl RenderThreadHandle {
    pub fn new(instance: Instance, adapter: Adapter, device: Device, queue: Queue) -> Self {
        let main_parker = Arc::new(Parker::new());
        let main_unparker = main_parker.clone();

        let executor = CommandExecutor::new(device.clone(), adapter.memory_properties());

        let shared = Arc::new(SharedState {
            instance,
            adapter,
            device,
            surfaces: UnsafeRefCell::new(RenderSurfaces::new()),
            state: AtomicU8::new(PIPELINE_STATE_IDLE),
            graph: UnsafeRefCell::new(RenderGraph::default()),
            main_unparker,
            // render_textures: UnsafeRefCell::new(HashMap::new()),
            jobs: UnsafeRefCell::new(VecDeque::new()),
            fps_limiter: UnsafeRefCell::new(FpsLimiter::new(FpsLimit::UNLIMITED)),
            shutdown: AtomicBool::new(false),
            scheduler: UnsafeRefCell::new(executor),
        });

        let render_unparker = start_render_thread(shared.clone(), queue);

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

fn start_render_thread(shared: Arc<SharedState>, queue: Queue) -> Arc<Parker> {
    let parker = Arc::new(Parker::new());
    let unparker = parker.clone();

    let renderer = RenderThread::new(shared, queue);
    std::thread::spawn(move || {
        renderer.run(parker);
    });

    unparker
}

struct RenderThread {
    shared: Arc<SharedState>,
    queue: Queue,
    schedule: Vec<NodeLabel>,
    command_pool: CommandPool,
}

impl RenderThread {
    fn new(shared: Arc<SharedState>, queue: Queue) -> Self {
        let command_pool = shared.device.create_command_pool();

        Self {
            shared,
            queue,
            schedule: Vec::new(),
            command_pool,
        }
    }

    fn run(mut self, parker: Arc<Parker>) {
        let _span = trace_span!("RenderThread::run").entered();

        loop {
            if self.shared.shutdown.load(Ordering::Acquire) {
                return;
            }

            // Sleep until the main thread requests a render.
            parker.park();
            // The main thread must set the state to `RENDERING`
            // before it unparks the parker above.
            debug_assert_eq!(
                self.shared.state.load(Ordering::Acquire),
                PIPELINE_STATE_RENDERING
            );

            // SAFETY: The pipeline is in rendering state, the render thread
            // has full access to the state.
            unsafe {
                self.render();
            }

            // Signal to the main thread that the render thread is now
            // idle.
            self.shared
                .state
                .store(PIPELINE_STATE_IDLE, Ordering::Release);
            self.shared.main_unparker.unpark();
        }
    }

    unsafe fn render(&mut self) {
        let _span = trace_span!("RenderThread::render").entered();

        let mut surfaces = unsafe { self.shared.surfaces.borrow_mut() };
        let mut graph = unsafe { self.shared.graph.borrow_mut() };
        let mut scheduler = unsafe { self.shared.scheduler.borrow_mut() };

        if graph.has_changed {
            graph.has_changed = false;
            let render_passes = RenderGraphScheduler.schedule(&graph).unwrap();
            self.schedule = render_passes;
        }

        let mut surfaces_to_present = Vec::new();
        let mut image_avail_sems = Vec::new();
        let mut render_done_sems = Vec::new();

        let mut swapchain_textures = Vec::new();
        let mut outputs = ScratchBuffer::new(surfaces.len());
        for (window, surface) in surfaces.iter_mut() {
            let mut image_avail = self.shared.device.create_semaphore();
            let render_done = self.shared.device.create_semaphore();

            let surface_window = surface.window().clone();
            let output = surface.swapchain.acquire_next_image(&mut image_avail);
            let output = outputs.insert(output);

            image_avail_sems.push(image_avail);
            render_done_sems.push(render_done);

            let mut queue = scheduler.queue();
            let swapchain_texture = queue.import_texture(
                unsafe { core::mem::transmute::<&'_ _, &'static _>(output.texture()) },
                AccessFlags::empty(),
                output.texture().size(),
                output.texture().format(),
                TextureUsage::RENDER_ATTACHMENT,
                1,
            );

            let mut resources = HashMap::new();
            resources.insert(
                SlotLabel::SURFACE,
                SlotValueInner::TextureRef(&swapchain_texture),
            );

            for node in &self.schedule {
                let node = graph.get(*node).unwrap();

                let mut ctx = RenderContext {
                    render_target: RenderTarget::Window(*window),
                    queue: &mut queue,
                    resources: &mut resources,
                    resource_permissions: &node.permissions,
                    size: swapchain_texture.size(),
                    format: swapchain_texture.format(),
                };
                node.node.render(&mut ctx);
            }

            // surface.window().pre_present_notify();

            swapchain_textures.push(swapchain_texture);
            surfaces_to_present.push((surface_window, output));
        }

        let mut encoder = self.command_pool.create_encoder().unwrap();

        for (_, output) in &mut surfaces_to_present {
            encoder.insert_pipeline_barriers(&PipelineBarriers {
                buffer: &[],
                texture: &[TextureBarrier {
                    src_access: AccessFlags::empty(),
                    dst_access: AccessFlags::COLOR_ATTACHMENT_WRITE,
                    texture: output.texture(),
                }],
            });
        }

        let res = scheduler.execute(&mut encoder);

        for (_, output) in &mut surfaces_to_present {
            encoder.insert_pipeline_barriers(&PipelineBarriers {
                buffer: &[],
                texture: &[TextureBarrier {
                    src_access: AccessFlags::COLOR_ATTACHMENT_WRITE,
                    dst_access: AccessFlags::PRESENT,
                    texture: output.texture(),
                }],
            });
        }

        self.queue
            .submit(
                core::iter::once(encoder.finish()),
                QueueSubmit {
                    wait: &mut image_avail_sems,
                    wait_stage: PipelineStageFlags::TOP_OF_PIPE,
                    signal: &mut render_done_sems,
                },
            )
            .unwrap();

        for ((window, output), mut render_done) in
            surfaces_to_present.into_iter().zip(&mut render_done_sems)
        {
            window.pre_present_notify();
            output.present(&mut self.queue, &mut render_done);
        }

        self.queue.wait_idle();
        unsafe {
            self.command_pool.reset();
        }

        scheduler.destroy(res);
        drop(render_done_sems);
        drop(image_avail_sems);

        for texture in swapchain_textures {
            let mut queue = scheduler.queue();
            queue.remove_imported_texture(texture);
        }
    }
}

// unsafe fn execute_render(state: &mut State) {
//     let _span = trace_span!("render_frame").entered();

//     let mut surfaces = unsafe { state.shared.surfaces.borrow_mut() };
//     let mut graph = unsafe { state.shared.graph.borrow_mut() };
//     let mut mipmap = unsafe { state.shared.mipmap_generator.borrow_mut() };
//     let mut fps_limiter = unsafe { state.shared.fps_limiter.borrow_mut() };

//     let mut encoder = state
//         .shared
//         .device
//         .create_command_encoder(&CommandEncoderDescriptor { label: None });

//     let mut outputs = Vec::new();

//     if graph.has_changed {
//         graph.has_changed = false;
//         let render_passes = RenderGraphScheduler.schedule(&graph).unwrap();
//         state.schedule = render_passes;
//     }

//     for (window, surface) in surfaces.iter_mut() {
//         let mut image_avail = state.shared.device.create_semaphore();
//         let mut render_done = state.shared.device.create_semaphore();

//         let output = surface.swapchain.acquire_next_image(&mut image_avail);

//         let mut queue = state.scheduler.queue();

//         let swapchain_texture = queue.import_texture(&output.texture(), AccessFlags::empty());

//         let mut resources = HashMap::new();
//         resources.insert(
//             SlotLabel::SURFACE,
//             SlotValueInner::TextureRef(&output.texture),
//         );

//         for node in &state.schedule {
//             let node = graph.get(*node).unwrap();

//             let mut ctx = RenderContext {
//                 render_target: RenderTarget::Window(*window),
//                 queue,
//                 resources: &mut resources,
//                 resource_permissions: &node.permissions,
//             };

//             node.node.render(&mut ctx);
//         }

//         outputs.push((surface, output));
//     }

//     let mut render_textures = unsafe { state.shared.render_textures.borrow_mut() };
//     for (id, render_texture) in render_textures.iter_mut() {
//         let texture = render_texture.texture.get_or_insert_with(|| {
//             state.shared.device.create_texture(&TextureDescriptor {
//                 label: None,
//                 size: Extent3d {
//                     width: render_texture.size.x,
//                     height: render_texture.size.y,
//                     depth_or_array_layers: 1,
//                 },
//                 mip_level_count: 1,
//                 sample_count: 1,
//                 dimension: TextureDimension::D2,
//                 format: TextureFormat::Rgba8Unorm,
//                 usage: TextureUsages::COPY_SRC | TextureUsages::RENDER_ATTACHMENT,
//                 view_formats: &[],
//             })
//         });

//         let target = texture.create_view(&TextureViewDescriptor::default());

//         let mut resources = HashMap::new();
//         resources.insert(SlotLabel::SURFACE, SlotValueInner::TextureRef(texture));

//         for node in &state.schedule {
//             let node = graph.get(*node).unwrap();

//             let mut ctx = RenderContext {
//                 render_target: RenderTarget::Image(*id),
//                 encoder: &mut encoder,
//                 size: render_texture.size,
//                 target: &target,
//                 format: texture.format(),
//                 device: &state.shared.device,
//                 queue: &state.shared.queue,
//                 mipmap: &mut mipmap,
//                 resources: &mut resources,
//                 resource_permissions: &node.permissions,
//             };

//             node.node.render(&mut ctx);
//         }
//     }

//     let mut mapping_buffers = Vec::new();

//     let mut jobs = unsafe { state.shared.jobs.borrow_mut() };
//     for job in jobs.drain(..) {
//         match job {
//             Job::SetFpsLimit(limit) => {
//                 *fps_limiter = FpsLimiter::new(limit);
//             }
//             Job::TextureToBuffer(id, tx) => {
//                 let texture = render_textures.get(&id).unwrap();

//                 // bytes_per_row must be aligned as required by wgpu.
//                 // 4 for RGBA8
//                 let mut bytes_per_row = 4 * texture.size.x;
//                 if bytes_per_row & COPY_BYTES_PER_ROW_ALIGNMENT != 0 {
//                     bytes_per_row &= u32::MAX & !COPY_BYTES_PER_ROW_ALIGNMENT;
//                     bytes_per_row += COPY_BYTES_PER_ROW_ALIGNMENT;
//                 }

//                 let buffer_size = bytes_per_row * texture.size.y;

//                 let buffer = state.shared.device.create_buffer(&BufferDescriptor {
//                     size: buffer_size as BufferAddress,
//                     usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
//                     mapped_at_creation: false,
//                     label: None,
//                 });

//                 encoder.copy_texture_to_buffer(
//                     ImageCopyTexture {
//                         aspect: TextureAspect::All,
//                         mip_level: 0,
//                         origin: Origin3d::ZERO,
//                         texture: texture.texture.as_ref().unwrap(),
//                     },
//                     ImageCopyBuffer {
//                         buffer: &buffer,
//                         layout: ImageDataLayout {
//                             offset: 0,
//                             bytes_per_row: Some(bytes_per_row),
//                             rows_per_image: None,
//                         },
//                     },
//                     Extent3d {
//                         width: texture.size.x,
//                         height: texture.size.y,
//                         depth_or_array_layers: 1,
//                     },
//                 );

//                 mapping_buffers.push((buffer, tx));
//             }
//         }
//     }

//     state.shared.queue.submit(std::iter::once(encoder.finish()));

//     fps_limiter.block_until_ready();

//     for (surface, output) in outputs {
//         surface.window().pre_present_notify();
//         output.present();
//     }

//     for (buffer, tx) in mapping_buffers {
//         // Unfortunately we need to wrap `Buffer` in `Arc` to be able
//         // to call `map_async` on the same value that takes a closure
//         // that also moves the value.
//         let buffer = Arc::new(buffer);

//         buffer
//             .clone()
//             .slice(..)
//             .map_async(MapMode::Read, move |res| {
//                 res.unwrap();

//                 {
//                     let slice = buffer.slice(..);
//                     let data = slice.get_mapped_range();
//                     let _ = tx.send(data.to_vec());
//                 }

//                 buffer.unmap();
//             });
//     }
// }

// #[derive(Debug)]
// pub(crate) struct RenderImageGpu {
//     pub(crate) size: UVec2,
//     /// Texture if initiliazed.
//     pub(crate) texture: Option<Texture>,
// }
