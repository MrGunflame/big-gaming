use std::collections::HashMap;

use game_window::windows::{WindowId, WindowState};
use glam::UVec2;
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};

use crate::backend::vulkan::{Adapter, Device, Instance, Surface, Swapchain};
use crate::backend::{PresentMode, SwapchainConfig, TextureFormat};

#[derive(Debug, Default)]
pub struct RenderSurfaces {
    windows: HashMap<WindowId, SurfaceData>,
}

impl RenderSurfaces {
    pub fn new() -> Self {
        Self {
            windows: HashMap::new(),
        }
    }

    pub fn create(
        &mut self,
        instance: &Instance,
        adapter: &Adapter,
        device: &Device,
        window: WindowState,
        id: WindowId,
    ) {
        let surfce = create_surface(window, instance, adapter, device).unwrap();
        self.windows.insert(id, surfce);
    }

    pub fn resize(&mut self, id: WindowId, device: &Device, size: UVec2) {
        let Some(surface) = self.windows.get_mut(&id) else {
            return;
        };

        resize_surface(surface, device, size);
    }

    pub fn get(&self, id: WindowId) -> Option<&SurfaceData> {
        self.windows.get(&id)
    }

    pub fn destroy(&mut self, id: WindowId) {
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
pub struct SurfaceData {
    pub surface: Surface,
    pub swapchain: Swapchain,
    pub config: SwapchainConfig,
    /// A handle to the window underlying the `surface`.
    ///
    /// NOTE: The surface MUST be dropped before the handle to the window is dropped.
    window: WindowState,
}

impl SurfaceData {
    /// Returns a handle to the window of the surface.
    // Note: It is important that the `self.window` value that never changes
    // after the `SurfaceData` is created.
    // To prevent acidental moving out of `self.window` we only return a reference
    // to the `WindowState` and keep the field itself as private.
    #[inline]
    pub fn window(&self) -> &WindowState {
        &self.window
    }
}

fn create_surface(
    window: WindowState,
    instance: &Instance,
    adapter: &Adapter,
    device: &Device,
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

    let caps = surface.get_capabilities(device);

    let Some(format) = get_surface_format(&caps.formats) else {
        tracing::error!("failed to select format for render suface");
        return Err(());
    };

    let Some(present_mode) = get_surface_present_mode(&caps.present_modes) else {
        tracing::error!("failed to select present mode for render surface");
        return Err(());
    };

    let config = SwapchainConfig {
        image_count: caps.min_images,
        extent: size,
        format,
        present_mode,
    };

    let swapchain = surface.create_swapchain(device, config, &caps);

    Ok(SurfaceData {
        swapchain,
        surface,
        config,
        window,
    })
}

fn resize_surface(surface: &mut SurfaceData, device: &Device, size: UVec2) {
    if size.x == 0 || size.y == 0 {
        return;
    }

    let caps = surface.surface.get_capabilities(device);
    surface.config.extent = size;
    surface.swapchain.recreate(surface.config, &caps);
}

fn get_surface_format(formats: &[TextureFormat]) -> Option<TextureFormat> {
    for format in formats {
        if !format.is_srgb() {
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
