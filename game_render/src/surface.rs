use std::collections::{HashMap, VecDeque};

use game_window::windows::{WindowId, WindowState};
use glam::UVec2;
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};

use crate::api::executor::TemporaryResources;
use crate::api::Texture;
use crate::backend::vulkan::{
    CommandPool, Device, Fence, Instance, Queue, Semaphore, Surface, Swapchain,
};
use crate::backend::{
    ColorSpace, PresentMode, SurfaceFormat, SwapchainCapabilities, SwapchainConfig,
};
use crate::fps_limiter::FpsLimiter;
use crate::FpsLimit;

// TODO: Make this configurable.
// Note that this value still bounded by the maxImageCount of the swapchain.
pub const MAX_FRAMES_IN_FLIGHT: u32 = 2;

#[derive(Copy, Clone, Debug)]
pub struct SurfaceConfig {
    pub size: UVec2,
}

#[derive(Debug, Default)]
pub(crate) struct RenderSurfaces {
    windows: HashMap<WindowId, SurfaceData>,
}

impl RenderSurfaces {
    pub(crate) fn new() -> Self {
        Self {
            windows: HashMap::new(),
        }
    }

    pub fn create(
        &mut self,
        instance: &Instance,
        device: &Device,
        queue: &Queue,
        window: WindowState,
        id: WindowId,
    ) {
        let surfce = create_surface(window, instance, device, queue).unwrap();
        self.windows.insert(id, surfce);
    }

    /// Resizes and reconfigure a surface.
    ///
    /// # Safety
    ///
    /// This will invalidate the current swapchain and commands accessing it must have been
    /// completed.
    pub unsafe fn resize(&mut self, id: WindowId, device: &Device, queue: &Queue, size: UVec2) {
        let Some(surface) = self.windows.get_mut(&id) else {
            return;
        };

        resize_surface(surface, device, queue, size);
    }

    pub fn get(&self, id: WindowId) -> Option<&SurfaceData> {
        self.windows.get(&id)
    }

    pub fn get_mut(&mut self, id: WindowId) -> Option<&mut SurfaceData> {
        self.windows.get_mut(&id)
    }

    /// Destroys a surface.
    ///
    /// # Safety
    ///
    /// This will invalidate the current swapchain and commands accessing it must have been
    /// completed.
    pub unsafe fn destroy(&mut self, id: WindowId) {
        self.windows.remove(&id);
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&WindowId, &mut SurfaceData)> {
        self.windows.iter_mut()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&WindowId, &SurfaceData)> {
        self.windows.iter()
    }
}

#[derive(Debug)]
pub(crate) struct SurfaceData {
    pub surface: Surface,
    pub swapchain: Swapchain,
    pub config: SwapchainConfig,
    pub next_frame: usize,
    pub frames: Vec<SurfaceFrameData>,
    pub limiter: FpsLimiter,
    /// Number of frames that have been submitted without having completed yet.
    pub frames_in_flight: VecDeque<usize>,
    /// A handle to the window underlying the `surface`.
    ///
    /// NOTE: The surface MUST be dropped before the handle to the window is dropped.
    pub window: WindowState,
}

#[derive(Debug)]
pub struct SurfaceFrameData {
    pub image_avail: Semaphore,
    pub render_done: Semaphore,
    /// Fence that becomes ready once the submission is done.
    pub submit_done: Fence,
    pub submit_done_used: bool,
    /// Fence that becomes ready once the presentation is done.
    pub present_done: Fence,
    pub present_done_used: bool,
    pub command_pool: CommandPool,
    pub resources: Option<TemporaryResources>,
    pub swapchain_texture: Option<Texture>,
}

fn create_surface(
    window: WindowState,
    instance: &Instance,
    device: &Device,
    queue: &Queue,
) -> Result<SurfaceData, ()> {
    let size = window.inner_size();

    let surface = unsafe {
        instance
            .create_surface(
                window.raw_display_handle().unwrap(),
                window.raw_window_handle().unwrap(),
            )
            .unwrap()
    };

    let caps = surface.get_capabilities(device, queue).unwrap();
    let config = create_swapchain_config(&caps, size);
    let swapchain = surface.create_swapchain(device, config, &caps).unwrap();

    let frames = (0..config.image_count)
        .map(|_| SurfaceFrameData {
            image_avail: device.create_semaphore().unwrap(),
            render_done: device.create_semaphore().unwrap(),
            submit_done: device.create_fence().unwrap(),
            submit_done_used: false,
            present_done: device.create_fence().unwrap(),
            present_done_used: false,
            command_pool: device.create_command_pool(queue.family().id).unwrap(),
            resources: None,
            swapchain_texture: None,
        })
        .collect();

    Ok(SurfaceData {
        swapchain,
        surface,
        config,
        window,
        next_frame: 0,
        frames,
        limiter: FpsLimiter::new(FpsLimit::UNLIMITED),
        frames_in_flight: VecDeque::with_capacity(MAX_FRAMES_IN_FLIGHT as usize),
    })
}

fn resize_surface(surface: &mut SurfaceData, device: &Device, queue: &Queue, size: UVec2) {
    if size.x == 0 || size.y == 0 {
        return;
    }

    let caps = surface.surface.get_capabilities(device, queue).unwrap();
    let config = create_swapchain_config(&caps, size);

    if surface.config.image_count != config.image_count {
        surface.next_frame = 0;

        for frame in &mut surface.frames {
            frame.present_done_used = false;
            frame.submit_done_used = false;
        }

        surface
            .frames
            .resize_with(config.image_count as usize, || SurfaceFrameData {
                image_avail: device.create_semaphore().unwrap(),
                render_done: device.create_semaphore().unwrap(),
                submit_done: device.create_fence().unwrap(),
                submit_done_used: false,
                present_done: device.create_fence().unwrap(),
                present_done_used: false,
                command_pool: device.create_command_pool(queue.family().id).unwrap(),
                resources: None,
                swapchain_texture: None,
            });
    }

    unsafe {
        surface.swapchain.recreate(config, &caps).unwrap();
    }

    surface.config = config;
}

fn create_swapchain_config(caps: &SwapchainCapabilities, surface_size: UVec2) -> SwapchainConfig {
    let image_count = MAX_FRAMES_IN_FLIGHT.clamp(
        caps.min_images,
        caps.max_images.map(|v| v.get()).unwrap_or(u32::MAX),
    );
    let extent = surface_size.clamp(caps.min_extent, caps.max_extent);
    let format = get_surface_format(&caps.formats).unwrap();
    let present_mode = get_surface_present_mode(&caps.present_modes).unwrap();

    SwapchainConfig {
        image_count,
        extent,
        format,
        present_mode,
    }
}

fn get_surface_format(formats: &[SurfaceFormat]) -> Option<SurfaceFormat> {
    for format in formats {
        if format.color_space == ColorSpace::SrgbNonLinear && !format.format.is_srgb() {
            return Some(*format);
        }
    }

    None
}

fn get_surface_present_mode(modes: &[PresentMode]) -> Option<PresentMode> {
    // TODO: FIFO is always supported, but
    // support other (better) modes is beneficial.
    for mode in modes {
        // FIFO currently does not work on Wayland, so this is stuck
        // on immediate until https://github.com/MrGunflame/big-gaming/issues/220
        // is resolved.
        if *mode == PresentMode::Immediate {
            return Some(*mode);
        }
    }

    None
}
