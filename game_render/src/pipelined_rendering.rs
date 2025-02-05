use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::{mpsc, Arc};

use ash::vk::PipelineStageFlags;
use game_common::cell::UnsafeRefCell;
use game_tasks::park::Parker;
use game_tracing::trace_span;
use game_window::windows::{WindowId, WindowState};

use crate::api::{CommandExecutor, TextureRegion};
use crate::backend::vulkan::{Adapter, Device, Instance, Queue};
use crate::backend::{AccessFlags, QueueSubmit, TextureUsage};
use crate::camera::RenderTarget;
use crate::fps_limiter::{FpsLimit, FpsLimiter};
use crate::graph::scheduler::RenderGraphScheduler;
use crate::graph::{NodeLabel, RenderContext, RenderGraph, SlotLabel, SlotValueInner};
use crate::surface::{RenderSurfaces, SurfaceConfig};

#[derive(Clone, Debug)]
pub enum Command {
    CreateSurface(WindowState),
    UpdateSurface(WindowId, SurfaceConfig),
    DestroySurface(WindowId),
    UpdateSurfaceFpsLimit(WindowId, FpsLimit),
    Render(Arc<Parker>),
}

pub struct SharedState {
    pub instance: Instance,
    pub adapter: Adapter,
    pub device: Device,
    pub graph: UnsafeRefCell<RenderGraph>,
    pub scheduler: UnsafeRefCell<CommandExecutor>,
}

pub struct RenderThreadHandle {
    pub shared: Arc<SharedState>,
    tx: mpsc::Sender<Command>,
    // While `Pipeline` is not directly thread-unsafe, we make no guarantees
    // whether atomic operations hold up when dispatching renders from multiple
    // threads.
    _marker: PhantomData<*const ()>,
}

impl RenderThreadHandle {
    pub fn new(instance: Instance, adapter: Adapter, device: Device, queue: Queue) -> Self {
        let executor = CommandExecutor::new(device.clone(), adapter.memory_properties());

        let shared = Arc::new(SharedState {
            instance,
            adapter,
            device,
            graph: UnsafeRefCell::new(RenderGraph::default()),
            scheduler: UnsafeRefCell::new(executor),
        });

        let (tx, rx) = mpsc::channel();

        let renderer = RenderThread::new(shared.clone(), rx, queue);
        std::thread::spawn(move || {
            renderer.run();
        });

        Self {
            shared,
            _marker: PhantomData,
            tx,
        }
    }

    pub fn send(&mut self, cmd: Command) {
        self.tx.send(cmd).ok();
    }
}

struct RenderThread {
    shared: Arc<SharedState>,
    queue: Queue,
    schedule: Vec<NodeLabel>,
    rx: mpsc::Receiver<Command>,
    surfaces: RenderSurfaces,
}

impl RenderThread {
    fn new(shared: Arc<SharedState>, rx: mpsc::Receiver<Command>, queue: Queue) -> Self {
        Self {
            shared,
            queue,
            schedule: Vec::new(),
            rx,
            surfaces: RenderSurfaces::new(),
        }
    }

    fn run(mut self) {
        let _span = trace_span!("RenderThread::run").entered();

        while let Ok(cmd) = self.rx.recv() {
            match cmd {
                Command::CreateSurface(window) => {
                    let id = window.id();
                    self.surfaces.create(
                        &self.shared.instance,
                        &self.shared.adapter,
                        &self.shared.device,
                        &self.queue,
                        window,
                        id,
                    );
                }
                Command::UpdateSurface(id, config) => {
                    // Wait until all commands for this surface have completed.
                    self.wait_idle(id);

                    // SAFETY: We have waited for all commands on this surface
                    // to be completed.
                    unsafe {
                        self.surfaces
                            .resize(id, &self.shared.device, &self.queue, config.size);
                    }
                }
                Command::DestroySurface(id) => {
                    // Wait until all commands for this surface have completed.
                    self.wait_idle(id);

                    // SAFETY: We have waited for all commands on this surface
                    // to be completed.
                    unsafe {
                        self.surfaces.destroy(id);
                    }
                }
                Command::UpdateSurfaceFpsLimit(id, limit) => {
                    let surfaces = self.surfaces.get_mut(id).unwrap();
                    surfaces.limiter = FpsLimiter::new(limit);
                }
                Command::Render(parker) => {
                    unsafe {
                        self.render();
                    }

                    parker.unpark();
                }
            }
        }
    }

    fn wait_idle(&mut self, id: WindowId) {
        let surface = self.surfaces.get_mut(id).unwrap();

        for (fence, used) in &mut surface.ready {
            if *used {
                fence.wait(None).unwrap();
                *used = false;
            }
        }
    }

    unsafe fn render(&mut self) {
        let _span = trace_span!("RenderThread::render").entered();

        let mut graph = unsafe { self.shared.graph.borrow_mut() };
        let mut scheduler = unsafe { self.shared.scheduler.borrow_mut() };

        if graph.has_changed {
            graph.has_changed = false;
            let render_passes = RenderGraphScheduler.schedule(&graph).unwrap();
            self.schedule = render_passes;
        }

        for (window, surface) in self.surfaces.iter_mut() {
            let res = &mut surface.resources[surface.next_frame];
            let swapchain_texture_slot = &mut surface.swapchain_textures[surface.next_frame];
            let (ready, ready_used) = &mut surface.ready[surface.next_frame];
            let image_avail = &mut surface.image_avail[surface.next_frame];
            let render_done = &mut surface.render_done[surface.next_frame];
            let pool = &mut surface.command_pools[surface.next_frame];
            surface.next_frame = (surface.next_frame + 1) % surface.config.image_count as usize;

            surface.limiter.block_until_ready();

            // Wait until all commands are done in this "frame slot".
            if *ready_used {
                ready.wait(None);
            }

            // Destroy all resources that were required for the commands.
            scheduler.destroy(core::mem::take(res));
            if let Some(texture) = swapchain_texture_slot.take() {
                scheduler.queue().remove_imported_texture(texture);
            }
            unsafe {
                pool.reset();
            }

            let mut output = surface.swapchain.acquire_next_image(image_avail).unwrap();

            let mut queue = scheduler.queue();

            let access = if *ready_used {
                AccessFlags::PRESENT
            } else {
                AccessFlags::empty()
            };

            let swapchain_texture = queue.import_texture(
                unsafe { output.take_texture() },
                access,
                TextureUsage::RENDER_ATTACHMENT,
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

            scheduler.queue().transition_texture(
                &TextureRegion {
                    texture: &swapchain_texture,
                    mip_level: 0,
                },
                AccessFlags::PRESENT,
            );

            let mut encoder = pool.create_encoder().unwrap();
            *res = scheduler.execute(&mut encoder);
            *swapchain_texture_slot = Some(swapchain_texture);
            *ready_used = true;

            self.queue
                .submit(
                    core::iter::once(encoder.finish().unwrap()),
                    QueueSubmit {
                        wait: core::slice::from_mut(image_avail),
                        signal: core::slice::from_mut(render_done),
                        signal_fence: ready,
                    },
                )
                .unwrap();

            surface.window.pre_present_notify();

            // SAFETY:
            // We have manually inserted a barrier to transition the texture
            // to the `PRESENT` flag.
            unsafe {
                output.present(&mut self.queue, render_done).unwrap();
            }

            drop(output);
        }

        // After all render passes have run transition all swapchain textures
        // into the PRESENT mode.
        // for texture in &swapchain_textures {
        //     scheduler.queue().transition_texture(
        //         &TextureRegion {
        //             texture,
        //             mip_level: 0,
        //         },
        //         AccessFlags::PRESENT,
        //     );
        // }

        // self.queue
        //     .submit(
        //         core::iter::once(encoder.finish()),
        //         QueueSubmit {
        //             wait: &mut image_avail_sems,
        //             wait_stage: PipelineStageFlags::TOP_OF_PIPE,
        //             signal: &mut render_done_sems,
        //         },
        //     )
        //     .unwrap();

        // for ((window, output), mut render_done) in
        //     surfaces_to_present.into_iter().zip(&mut render_done_sems)
        // {
        //     window.pre_present_notify();
        //     output.present(&mut self.queue, &mut render_done);
        // }

        // self.queue.wait_idle();
        // unsafe {
        //     self.command_pool.reset();
        // }

        // scheduler.destroy(res);
        // drop(render_done_sems);
        // drop(image_avail_sems);

        // for texture in swapchain_textures {
        //     let mut queue = scheduler.queue();
        //     queue.remove_imported_texture(texture);
        // }
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
