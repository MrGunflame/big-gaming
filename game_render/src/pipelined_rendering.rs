use std::collections::{BTreeMap, HashMap};
use std::marker::PhantomData;
use std::sync::{mpsc, Arc};
use std::time::Duration;

use game_common::cell::UnsafeRefCell;
use game_tasks::park::Parker;
use game_tracing::trace_span;
use game_window::windows::{WindowId, WindowState};

use crate::api::queries::{ManagedQueryPool, QueryObject};
use crate::api::{CommandExecutor, TextureRegion};
use crate::backend::vulkan::{Adapter, Device, Instance, Queue};
use crate::backend::{AccessFlags, QueuePresent, QueueSubmit, TextureDescriptor, TextureUsage};
use crate::camera::RenderTarget;
use crate::fps_limiter::{FpsLimit, FpsLimiter};
use crate::graph::scheduler::RenderGraphScheduler;
use crate::graph::{NodeLabel, RenderContext, RenderGraph, SlotLabel, SlotValueInner};
use crate::statistics::{Pass, Statistics};
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
    pub device: Device,
    pub graph: UnsafeRefCell<RenderGraph>,
    pub scheduler: UnsafeRefCell<CommandExecutor>,
    pub statistics: Arc<Statistics>,
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
    pub fn new(
        instance: Instance,
        adapter: Adapter,
        device: Device,
        queue: Queue,
        statistics: Arc<Statistics>,
    ) -> Self {
        let executor = CommandExecutor::new(
            device.clone(),
            adapter.memory_properties(),
            statistics.clone(),
            adapter.properties(),
        );

        let shared = Arc::new(SharedState {
            instance,
            device,
            graph: UnsafeRefCell::new(RenderGraph::default()),
            scheduler: UnsafeRefCell::new(executor),
            statistics,
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

        // Wait until all queue submissions have completed.
        for frame in &mut surface.frames {
            if frame.present_done_used {
                frame.present_done.wait(None).unwrap();
                frame.present_done_used = false;
            }
        }

        // Wait until all presentations have completed.
        // Only then is it safe to destroy the swapchain.
        // Note that if we have `VK_EXT_swapchain_maintenance1` we can wait
        // for the present fence to be signaled and all is well.
        // If we do not have this extension it is technically never safe to
        // destroy the swapchain. Waiting for the device to become idle is
        // the best we can do.
        // See https://github.com/KhronosGroup/Vulkan-Docs/issues/1678
        if self.shared.device.extensions().swapchain_maintenance1 {
            for frame in &mut surface.frames {
                if frame.present_done_used {
                    frame.present_done.wait(None).unwrap();
                    frame.present_done_used = false;
                }
            }
        } else {
            unsafe {
                self.shared.device.wait_idle();
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

        let mut image_avail = self.shared.device.create_semaphore().unwrap();

        for (window, surface) in self.surfaces.iter_mut() {
            surface.limiter.block_until_ready();

            // Acquire the next swapchain image.
            // Note that there are no guarantees about the index of the acquired image.
            let mut output = surface
                .swapchain
                .acquire_next_image(&mut image_avail)
                .unwrap();

            let frame = &mut surface.frames[output.index() as usize];

            if frame.submit_done_used {
                frame.submit_done.wait(None).unwrap();
            }

            if let Some(query_pool) = &frame.resources.query_pool {
                unsafe {
                    record_query_statistics(
                        &self.shared.device,
                        &self.queue,
                        query_pool,
                        &self.shared.statistics,
                    );
                }
            }

            // Destroy all resources that were required for the commands.
            scheduler.destroy(core::mem::take(&mut frame.resources));
            if let Some(texture) = frame.swapchain_texture.take() {
                scheduler.queue().remove_imported_texture(texture);
            }

            unsafe {
                frame.command_pool.reset().unwrap();
            }

            let mut queue = scheduler.queue();

            let swapchain_texture = unsafe {
                let texture = output.take_texture();
                let descriptor = TextureDescriptor {
                    size: texture.size(),
                    mip_levels: texture.mip_levels(),
                    format: texture.format(),
                    usage: TextureUsage::RENDER_ATTACHMENT,
                };

                queue.import_texture(texture, descriptor, AccessFlags::empty())
            };

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

            let mut encoder = frame.command_pool.create_encoder().unwrap();
            frame.resources = scheduler.execute(&mut encoder);
            frame.swapchain_texture = Some(swapchain_texture);
            frame.submit_done_used = true;

            core::mem::swap(&mut frame.image_avail, &mut image_avail);

            self.queue
                .submit(
                    core::iter::once(encoder.finish().unwrap()),
                    QueueSubmit {
                        wait: core::slice::from_mut(&mut frame.image_avail),
                        signal: core::slice::from_mut(&mut frame.render_done),
                        signal_fence: &mut frame.submit_done,
                    },
                )
                .unwrap();

            surface.window.pre_present_notify();

            if self.shared.device.extensions().swapchain_maintenance1 {
                if frame.present_done_used {
                    frame.present_done.wait(None).unwrap();
                }

                frame.present_done_used = true;
            }

            // SAFETY:
            // We have manually inserted a barrier to transition the texture
            // to the `PRESENT` flag.
            unsafe {
                output
                    .present(
                        &mut self.queue,
                        QueuePresent {
                            wait: &mut frame.render_done,
                            signal: Some(&mut frame.present_done),
                        },
                    )
                    .unwrap();
            }

            drop(output);
        }
    }
}

unsafe fn record_query_statistics(
    device: &Device,
    queue: &Queue,
    query_pool: &ManagedQueryPool,
    statistics: &Statistics,
) {
    let factor = device.limits().timestamp_period_nanos;
    let timestamp_bits = queue.family().timestamp_bits;

    let mut passes = Vec::new();
    let mut started_passes = BTreeMap::new();

    let mut timestamps = unsafe { query_pool.get() };
    for timestamp in &mut timestamps {
        if timestamp_bits < size_of::<u64>() as u32 {
            // Only the first starting bits are valid.
            *timestamp &= (1 << timestamp_bits) - 1;
        }
    }

    let mut submit_start = 0;
    let mut submit_end = 0;

    for (object, timestamp) in query_pool.objects.iter().zip(&timestamps) {
        match object {
            QueryObject::BeginCommands => {
                submit_start = *timestamp;
            }
            QueryObject::EndCommands => {
                submit_end = *timestamp;
            }
            QueryObject::BeginPass(name) => {
                started_passes.insert(name, *timestamp);
            }
            QueryObject::EndPass(name) => {
                let Some(start) = started_passes.remove(name) else {
                    continue;
                };

                let end = *timestamp;

                let elapsed = end.wrapping_sub(start) as f32 * factor;
                let time = Duration::from_nanos(elapsed as u64);

                passes.push(Pass { name, time });
            }
        }
    }

    let elapsed = submit_end.wrapping_sub(submit_start) as f32 * factor;
    let time = Duration::from_nanos(elapsed as u64);

    let mut timings = statistics.gpu_timings.write();
    timings.time = time;
    timings.passes = passes;
}
