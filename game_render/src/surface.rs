use std::collections::HashMap;

use game_window::windows::{WindowId, WindowState};
use glam::UVec2;
use wgpu::{
    Adapter, CompositeAlphaMode, Device, Instance, PresentMode, Surface, SurfaceConfiguration,
    TextureFormat, TextureUsages,
};

use crate::depth_stencil::DepthData;

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
        let surfce = create_surface(window, &instance, &adapter, &device).unwrap();
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
    pub config: SurfaceConfiguration,
    pub depth: DepthData,
    /// A handle to the window underlying the `surface`.
    ///
    /// NOTE: The surface MUST be dropped before the handle to the window is dropped.
    _window: WindowState,
}

fn create_surface(
    window: WindowState,
    instance: &Instance,
    adapter: &Adapter,
    device: &Device,
) -> Result<SurfaceData, ()> {
    let size = window.inner_size();

    let surface = match unsafe { instance.create_surface(&window) } {
        Ok(surface) => surface,
        Err(err) => {
            tracing::error!("failed to create surface: {}", err);
            return Err(());
        }
    };

    let caps = surface.get_capabilities(adapter);

    let Some(format) = get_surface_format(&caps.formats) else {
        tracing::error!("failed to select format for render suface");
        return Err(());
    };

    let Some(present_mode) = get_surface_present_mode(&caps.present_modes) else {
        tracing::error!("failed to select present mode for render surface");
        return Err(());
    };

    let Some(alpha_mode) = get_surface_alpha_mode(&caps.alpha_modes) else {
        tracing::error!("failed to select alpha mode for render surface");
        return Err(());
    };

    let config = SurfaceConfiguration {
        usage: TextureUsages::RENDER_ATTACHMENT,
        format,
        width: size.x,
        height: size.y,
        present_mode,
        alpha_mode,
        view_formats: vec![],
    };

    surface.configure(&device, &config);

    Ok(SurfaceData {
        surface,
        config,
        _window: window,
        depth: DepthData::new(device, size),
    })
}

fn resize_surface(surface: &mut SurfaceData, device: &Device, size: UVec2) {
    if size.x == 0 || size.y == 0 {
        return;
    }

    surface.config.width = size.x;
    surface.config.height = size.y;
    surface.surface.configure(device, &surface.config);

    surface.depth = DepthData::new(device, size);
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
        match mode {
            PresentMode::Fifo => return Some(*mode),
            _ => (),
        }
    }

    None
}

fn get_surface_alpha_mode(modes: &[CompositeAlphaMode]) -> Option<CompositeAlphaMode> {
    modes.get(0).copied()
}
