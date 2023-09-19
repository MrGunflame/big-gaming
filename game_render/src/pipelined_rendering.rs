use std::sync::{mpsc, Arc};

use game_tracing::trace_span;
use parking_lot::Mutex;
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
    instance: Instance,
    adapter: Adapter,
    device: Device,
    queue: Queue,
    surfaces: Mutex<RenderSurfaces>,
    mipmap_generator: MipMapGenerator,
    state: Mutex<PipelineState>,
    graph: Mutex<RenderGraph>,
}

pub struct Pipeline {
    shared: Arc<SharedState>,
    tx: mpsc::Sender<()>,
}

impl Pipeline {
    pub fn is_idle(&self) -> bool {
        *self.shared.state.lock() == PipelineState::Idle
    }
}

impl Pipeline {
    pub fn new(instance: Instance, adapter: Adapter, device: Device, queue: Queue) -> Self {
        let shared = Arc::new(SharedState {
            mipmap_generator: MipMapGenerator::new(&device),
            instance,
            adapter,
            device,
            queue,
            surfaces: Mutex::new(RenderSurfaces::new()),
            state: Mutex::new(PipelineState::Idle),
            graph: Mutex::new(RenderGraph::default()),
        });

        let tx = start_render_thread(shared.clone());

        Self { shared, tx }
    }

    pub fn update(&mut self) {
        debug_assert!(self.is_idle());
    }

    pub fn render(&mut self) {
        // Wait until render thread is ready.
        while !self.is_idle() {
            std::thread::sleep_ms(1);
        }

        let _ = self.tx.send(());
    }
}

fn start_render_thread(shared: Arc<SharedState>) -> mpsc::Sender<()> {
    let (tx, rx) = mpsc::channel();

    std::thread::spawn(move || {
        while let Ok(()) = rx.recv() {
            *shared.state.lock() = PipelineState::Rendering;

            let _span = trace_span!("render_frame").entered();

            let surfaces = shared.surfaces.lock();
            let graph = shared.graph.lock();

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
                };

                for node in &graph.nodes {
                    node.render(&mut ctx);
                }

                shared.queue.submit(std::iter::once(encoder.finish()));
                output.present();
            }

            *shared.state.lock() = PipelineState::Idle;
        }
    });

    tx
}
