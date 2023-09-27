use std::sync::Arc;

use crossbeam::sync::{Parker, Unparker};
use game_common::cell::UnsafeRefCell;
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
    pub instance: Instance,
    pub adapter: Adapter,
    pub device: Device,
    pub queue: Queue,
    pub surfaces: UnsafeRefCell<RenderSurfaces>,
    mipmap_generator: UnsafeRefCell<MipMapGenerator>,
    state: Mutex<PipelineState>,
    pub graph: UnsafeRefCell<RenderGraph>,
    /// Unparker for the calling thread.
    main_unparker: Unparker,
}

pub struct Pipeline {
    pub shared: Arc<SharedState>,
    main_parker: Parker,
    /// Unparker for the render thread.
    render_unparker: Unparker,
}

impl Pipeline {
    pub fn new(instance: Instance, adapter: Adapter, device: Device, queue: Queue) -> Self {
        let main_parker = Parker::new();
        let main_unparker = main_parker.unparker().clone();

        let shared = Arc::new(SharedState {
            mipmap_generator: UnsafeRefCell::new(MipMapGenerator::new(&device)),
            instance,
            adapter,
            device,
            queue,
            surfaces: UnsafeRefCell::new(RenderSurfaces::new()),
            state: Mutex::new(PipelineState::Idle),
            graph: UnsafeRefCell::new(RenderGraph::default()),
            main_unparker,
        });

        let render_unparker = start_render_thread(shared.clone());

        Self {
            shared,
            render_unparker,
            main_parker,
        }
    }

    pub fn is_idle(&self) -> bool {
        *self.shared.state.lock() == PipelineState::Idle
    }

    pub fn wait_idle(&self) {
        let _span = trace_span!("Pipeline::wait_idle").entered();

        while *self.shared.state.lock() != PipelineState::Idle {
            self.main_parker.park();
        }
    }

    /// # Safety
    ///
    /// renderer must be idle.
    pub unsafe fn render_unchecked(&mut self) {
        debug_assert!(self.is_idle());

        *self.shared.state.lock() = PipelineState::Rendering;
        self.render_unparker.unpark();
    }
}

fn start_render_thread(shared: Arc<SharedState>) -> Unparker {
    let parker = Parker::new();
    let unparker = parker.unparker().clone();

    std::thread::spawn(move || loop {
        while *shared.state.lock() != PipelineState::Rendering {
            parker.park();
        }

        // SAFETY: The pipeline is in rendering state, the render thread
        // has full access to the state.
        unsafe {
            execute_render(&shared);
        }

        *shared.state.lock() = PipelineState::Idle;
        shared.main_unparker.unpark();
    });

    unparker
}

unsafe fn execute_render(shared: &SharedState) {
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
}
