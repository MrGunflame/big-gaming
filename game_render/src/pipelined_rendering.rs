use std::sync::{mpsc, Arc};

use game_common::cell::UnsafeRefCell;
use game_tracing::trace_span;
use parking_lot::{Condvar, Mutex};
use wgpu::{Adapter, CommandEncoderDescriptor, Device, Instance, Queue, TextureViewDescriptor};

use crate::graph::{RenderContext, RenderGraph};
use crate::mipmap::MipMapGenerator;
use crate::surface::RenderSurfaces;

// TODO: We can likely replace this with atomics if we're careful.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum PipelineState {
    /// The renderer is in the process of doing a render pass.
    Rendering,
    /// The renderer is waiting for work.
    Idle,
}

pub struct SharedState {
    pub instance: Instance,
    pub adapter: Adapter,
    pub device: Device,
    pub queue: Queue,
    pub surfaces: UnsafeRefCell<RenderSurfaces>,
    pub mipmap_generator: UnsafeRefCell<MipMapGenerator>,
    state: Mutex<PipelineState>,
    pub graph: UnsafeRefCell<RenderGraph>,
    unparker: Condvar,
}

pub struct Pipeline {
    pub shared: Arc<SharedState>,
    tx: mpsc::Sender<()>,
}

impl Pipeline {
    pub fn new(instance: Instance, adapter: Adapter, device: Device, queue: Queue) -> Self {
        let shared = Arc::new(SharedState {
            mipmap_generator: UnsafeRefCell::new(MipMapGenerator::new(&device)),
            instance,
            adapter,
            device,
            queue,
            surfaces: UnsafeRefCell::new(RenderSurfaces::new()),
            state: Mutex::new(PipelineState::Idle),
            graph: UnsafeRefCell::new(RenderGraph::default()),
            unparker: Condvar::new(),
        });

        let tx = start_render_thread(shared.clone());

        Self { shared, tx }
    }

    pub fn is_idle(&self) -> bool {
        *self.shared.state.lock() == PipelineState::Idle
    }

    pub fn wait_idle(&self) {
        let _span = trace_span!("Pipeline::wait_idle").entered();

        let mut state = self.shared.state.lock();
        while *state != PipelineState::Idle {
            self.shared.unparker.wait(&mut state);
        }
    }

    /// # Safety
    ///
    /// renderer must be idle.
    pub unsafe fn render_unchecked(&mut self) {
        debug_assert!(self.is_idle());

        *self.shared.state.lock() = PipelineState::Rendering;
        let _ = self.tx.send(());
    }
}

fn start_render_thread(shared: Arc<SharedState>) -> mpsc::Sender<()> {
    let (tx, rx) = mpsc::channel();

    std::thread::spawn(move || {
        while let Ok(()) = rx.recv() {
            // The caller must transition the state to `Rendering`.
            debug_assert!(*shared.state.lock() == PipelineState::Rendering);

            let _span = trace_span!("render_frame").entered();

            let surfaces = unsafe { shared.surfaces.get() };
            let graph = unsafe { shared.graph.get() };
            let mut mipmap = unsafe { shared.mipmap_generator.get_mut() };

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

                let mut encoder = shared
                    .device
                    .create_command_encoder(&CommandEncoderDescriptor {
                        label: Some("render_encoder"),
                    });

                let mut ctx = RenderContext {
                    window: *window,
                    encoder: &mut encoder,
                    width: output.texture.width(),
                    height: output.texture.height(),
                    target: &target,
                    surface: &surface,
                    format: surface.config.format,
                    device: &shared.device,
                    queue: &shared.queue,
                    mipmap: &mut mipmap,
                };

                for node in &graph.nodes {
                    node.render(&mut ctx);
                }

                shared.queue.submit(std::iter::once(encoder.finish()));
                output.present();
            }

            *shared.state.lock() = PipelineState::Idle;
            shared.unparker.notify_one();
        }
    });

    tx
}
