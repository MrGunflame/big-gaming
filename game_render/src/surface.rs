use std::collections::HashMap;

use game_window::windows::{WindowId, WindowState};
use glam::UVec2;
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};

use crate::api::executor::TemporaryResources;
use crate::backend::vulkan::{
    Adapter, CommandPool, Device, Fence, Instance, Queue, Semaphore, Surface, Swapchain,
};
use crate::backend::{
    ColorSpace, PresentMode, SurfaceFormat, SwapchainCapabilities, SwapchainConfig, TextureFormat,
};
use crate::fps_limiter::FpsLimiter;
use crate::FpsLimit;

// TODO: Make this configurable.
// Note that this value still bounded by the maxImageCount of the swapchain.
const MAX_FRAMES_IN_FLIGHT: u32 = 3;

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
        adapter: &Adapter,
        device: &Device,
        queue: &Queue,
        window: WindowState,
        id: WindowId,
    ) {
        let surfce = create_surface(window, instance, adapter, device, queue).unwrap();
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
    pub image_avail: Vec<Semaphore>,
    pub render_done: Vec<Semaphore>,
    pub ready: Vec<(Fence, bool)>,
    pub resources: Vec<TemporaryResources>,
    pub swapchain_textures: Vec<Option<crate::api::Texture>>,
    pub command_pools: Vec<CommandPool>,
    pub limiter: FpsLimiter,
    /// A handle to the window underlying the `surface`.
    ///
    /// NOTE: The surface MUST be dropped before the handle to the window is dropped.
    pub window: WindowState,
}

fn create_surface(
    window: WindowState,
    instance: &Instance,
    adapter: &Adapter,
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

    Ok(SurfaceData {
        swapchain,
        surface,
        config,
        window,
        image_avail: (0..config.image_count)
            .map(|_| device.create_semaphore().unwrap())
            .collect(),
        render_done: (0..config.image_count)
            .map(|_| device.create_semaphore().unwrap())
            .collect(),
        ready: (0..config.image_count)
            .map(|_| (device.create_fence().unwrap(), false))
            .collect(),
        next_frame: 0,
        resources: (0..config.image_count)
            .map(|_| TemporaryResources::default())
            .collect(),
        swapchain_textures: vec![(const { None }); config.image_count as usize],
        command_pools: (0..config.image_count)
            .map(|_| device.create_command_pool(queue.family()).unwrap())
            .collect(),
        limiter: FpsLimiter::new(FpsLimit::UNLIMITED),
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
        let len = config.image_count as usize;
        surface
            .image_avail
            .resize_with(len, || device.create_semaphore().unwrap());
        surface
            .render_done
            .resize_with(len, || device.create_semaphore().unwrap());
        surface
            .ready
            .resize_with(len, || (device.create_fence().unwrap(), false));
        for (_, used) in &mut surface.ready {
            *used = false;
        }

        surface.resources.resize_with(len, Default::default);
        surface.swapchain_textures.resize_with(len, || None);
        surface
            .command_pools
            .resize_with(len, || device.create_command_pool(queue.family()).unwrap());
    }

    unsafe {
        surface.swapchain.recreate(config, &caps);
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
