use std::marker::PhantomData;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

use game_common::cell::UnsafeRefCell;
use game_tasks::park::Parker;
use game_tracing::trace_span;
use wgpu::{Adapter, CommandEncoderDescriptor, Device, Instance, Queue, TextureViewDescriptor};

use crate::graph::{RenderContext, RenderGraph};
use crate::mipmap::MipMapGenerator;
use crate::surface::RenderSurfaces;

const PIPELINE_STATE_RENDERING: u8 = 1;
const PIPELINE_STATE_IDLE: u8 = 2;

pub struct SharedState {
    pub instance: Instance,
    pub adapter: Adapter,
    pub device: Device,
    pub queue: Queue,
    pub surfaces: UnsafeRefCell<RenderSurfaces>,
    mipmap_generator: UnsafeRefCell<MipMapGenerator>,
    pub graph: UnsafeRefCell<RenderGraph>,
    state: AtomicU8,
    /// Unparker for the calling thread.
    main_unparker: Arc<Parker>,
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
}

fn start_render_thread(shared: Arc<SharedState>) -> Arc<Parker> {
    let parker = Arc::new(Parker::new());
    let unparker = parker.clone();

    std::thread::spawn(move || loop {
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
            surface,
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
