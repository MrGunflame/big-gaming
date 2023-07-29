use std::collections::HashMap;

use bevy_ecs::prelude::{Entity, EventReader};
use bevy_ecs::removal_detection::RemovedComponents;
use bevy_ecs::system::{Query, Res, ResMut, Resource};
use bevy_ecs::world::World;
use game_window::events::{WindowCreated, WindowResized};
use game_window::{Window, WindowState};
use wgpu::{
    CommandEncoderDescriptor, CompositeAlphaMode, PresentMode, Surface, SurfaceConfiguration,
    TextureFormat, TextureUsages, TextureViewDescriptor,
};

use crate::graph::{RenderContext, RenderGraph};
use crate::{RenderAdapter, RenderDevice, RenderInstance, RenderQueue};

#[derive(Debug, Default, Resource)]
pub struct RenderSurfaces {
    windows: HashMap<Entity, SurfaceData>,
}

impl RenderSurfaces {
    fn insert(&mut self, window: Entity, data: SurfaceData) {
        self.windows.insert(window, data);
    }

    fn get_mut(&mut self, window: Entity) -> &mut SurfaceData {
        self.windows.get_mut(&window).unwrap()
    }

    fn remove(&mut self, window: Entity) {
        self.windows.remove(&window);
    }

    fn iter_mut(&mut self) -> impl Iterator<Item = (Entity, &mut SurfaceData)> {
        self.windows.iter_mut().map(|(e, d)| (*e, d))
    }
}

#[derive(Debug)]
struct SurfaceData {
    surface: Surface,
    config: SurfaceConfiguration,
    /// A handle to the window underlying the `surface`.
    ///
    /// NOTE: The surface MUST be dropped before the handle to the window is dropped.
    _window: WindowState,
}

/// Create a render surface for newly created windows.
pub fn create_surfaces(
    instance: Res<RenderInstance>,
    adapter: Res<RenderAdapter>,
    device: Res<RenderDevice>,
    mut surfaces: ResMut<RenderSurfaces>,
    windows: Query<(Entity, &WindowState)>,
    mut events: EventReader<WindowCreated>,
) {
    for event in events.iter() {
        // Woops, seems like the window has already destroyed
        // again.
        let Ok((_, window)) = windows.get(event.window) else {
            continue;
        };

        let size = window.inner_size();

        let window = window.clone();
        let surface = match unsafe { instance.0.create_surface(&window) } {
            Ok(surface) => surface,
            Err(err) => {
                tracing::error!("failed to create render surface for window: {}", err);
                continue;
            }
        };

        let caps = surface.get_capabilities(&adapter);

        let Some(format) = get_surface_format(&caps.formats) else {
            tracing::error!("failed to select format for render suface");
            continue;
        };

        let Some(present_mode) = get_surface_present_mode(&caps.present_modes) else {
            tracing::error!("failed to select present mode for render surface");
            continue;
        };

        let Some(alpha_mode) = get_surface_alpha_mode(&caps.alpha_modes) else {
            tracing::error!("failed to select alpha mode for render surface");
            continue;
        };

        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width,
            height: size.height,
            present_mode,
            alpha_mode,
            view_formats: vec![],
        };

        surface.configure(&device, &config);

        surfaces.insert(
            event.window,
            SurfaceData {
                surface,
                config,
                _window: window,
            },
        );
    }
}

pub fn resize_surfaces(
    device: Res<RenderDevice>,
    mut surfaces: ResMut<RenderSurfaces>,
    mut events: EventReader<WindowResized>,
) {
    for event in events.iter() {
        if event.width == 0 || event.height == 0 {
            continue;
        }

        let surface = surfaces.get_mut(event.window);

        surface.config.width = event.width;
        surface.config.height = event.height;

        surface.surface.configure(&device, &surface.config);
    }
}

pub fn destroy_surfaces(
    mut surfaces: ResMut<RenderSurfaces>,
    mut destroyed_windows: RemovedComponents<Window>,
) {
    for entity in destroyed_windows.iter() {
        surfaces.remove(entity);
    }
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

pub fn render_to_surfaces(world: &mut World) {
    world.resource_scope::<RenderGraph, ()>(|world, mut render_graph| {
        for node in &mut render_graph.nodes {
            node.update(world);
        }
    });

    world.resource_scope::<RenderSurfaces, ()>(|world, mut surfaces| {
        let device: &RenderDevice = world.resource();
        let queue: &RenderQueue = world.resource();
        let render_graph: &RenderGraph = world.resource();

        for (entity, surface) in surfaces.iter_mut() {
            let output = match surface.surface.get_current_texture() {
                Ok(output) => output,
                Err(err) => {
                    tracing::warn!("surface error: {}", err);

                    continue;
                }
            };

            let target = output.texture.create_view(&TextureViewDescriptor {
                label: Some("surface_view"),
                format: Some(surface.config.format),
                ..Default::default()
            });

            let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
                label: Some("render_encoder"),
            });

            let ctx = RenderContext {
                window: entity,
                encoder: &mut encoder,
                target: &target,
                width: output.texture.width(),
                height: output.texture.height(),
            };

            for node in &render_graph.nodes {
                node.render(world, &mut ctx);
            }

            queue.submit(std::iter::once(encoder.finish()));
            output.present();
        }
    });
}
