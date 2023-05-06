pub mod buffer;
pub mod graph;
pub mod layout;

use std::collections::HashMap;
use std::sync::Arc;

use bevy_app::{App, Plugin};
use bevy_ecs::prelude::{Entity, EventReader};
use bevy_ecs::query::QueryState;
use bevy_ecs::system::{Query, Res, ResMut, Resource};
use bevy_ecs::world::World;
use game_window::events::{WindowCreated, WindowDestroyed, WindowResized};
use game_window::{WindowPlugin, WindowState};
use graph::{RenderContext, RenderGraph};
use wgpu::{
    Adapter, Backends, Color, CommandEncoderDescriptor, Device, DeviceDescriptor, Features,
    Instance, InstanceDescriptor, Limits, LoadOp, Operations, PowerPreference, Queue,
    RenderPassColorAttachment, RenderPassDescriptor, RequestAdapterOptions, Surface,
    SurfaceConfiguration, SurfaceError, TextureFormat, TextureUsages, TextureViewDescriptor,
};

#[derive(Copy, Clone, Debug, Default)]
pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(WindowPlugin);

        let instance = Instance::new(InstanceDescriptor {
            backends: Backends::all(),
            dx12_shader_compiler: Default::default(),
        });

        let adapter =
            futures_lite::future::block_on(instance.request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::default(),
                compatible_surface: None,
                force_fallback_adapter: false,
            }))
            .unwrap();

        let (device, queue) = futures_lite::future::block_on(adapter.request_device(
            &DeviceDescriptor {
                features: Features::empty(),
                limits: Limits::default(),
                label: None,
            },
            None,
        ))
        .unwrap();

        app.insert_resource(RenderInstance(instance));
        app.insert_resource(RenderAdapter(Arc::new(adapter)));
        app.insert_resource(RenderDevice(device));
        app.insert_resource(RenderQueue(Arc::new(queue)));
        app.insert_resource(WindowSurfaces::default());

        let render_graph = RenderGraph::default();
        app.insert_resource(render_graph);

        let query = WindowQuery(app.world.query::<&WindowState>());
        app.insert_resource(query);

        app.add_system(create_surfaces);
        app.add_system(destroy_surfaces);
        app.add_system(render_surfaces);
    }
}

#[derive(Debug, Resource)]
pub struct RenderInstance(pub Instance);

#[derive(Clone, Debug, Resource)]
pub struct RenderAdapter(pub Arc<Adapter>);

#[derive(Debug, Resource)]
pub struct RenderDevice(pub Device);

#[derive(Clone, Debug, Resource)]
pub struct RenderQueue(pub Arc<Queue>);

#[derive(Debug, Default, Resource)]
pub struct WindowSurfaces {
    windows: HashMap<Entity, SurfaceData>,
}

#[derive(Debug)]
struct SurfaceData {
    surface: Surface,
    format: TextureFormat,
    config: SurfaceConfiguration,
}

/// Create render surfaces for new windows.
pub fn create_surfaces(
    instance: Res<RenderInstance>,
    mut surfaces: ResMut<WindowSurfaces>,
    windows: Query<(Entity, &WindowState)>,
    mut events: EventReader<WindowCreated>,
    adapter: Res<RenderAdapter>,
    device: Res<RenderDevice>,
) {
    for event in events.iter() {
        let (_, window) = windows.get(event.window).unwrap();

        let size = window.0.inner_size();

        let surface = unsafe { instance.0.create_surface(&window.0) }.unwrap();

        let caps = surface.get_capabilities(&adapter.0);

        let format = caps
            .formats
            .iter()
            .copied()
            .filter(|f| f.is_srgb())
            .next()
            .unwrap_or(caps.formats[0]);

        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width,
            height: size.height,
            present_mode: caps.present_modes[0],
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
        };

        surface.configure(&device.0, &config);

        surfaces.windows.insert(
            event.window,
            SurfaceData {
                surface,
                format,
                config,
            },
        );
    }
}

pub fn resize_surfaces(
    mut surfaces: ResMut<WindowSurfaces>,
    mut events: EventReader<WindowResized>,
    device: Res<RenderDevice>,
) {
    for event in events.iter() {
        if event.width == 0 || event.height == 0 {
            continue;
        }

        let surface = surfaces.windows.get_mut(&event.window).unwrap();

        surface.config.width = event.width;
        surface.config.height = event.height;

        surface.surface.configure(&device.0, &surface.config);
    }
}

pub fn destroy_surfaces(
    mut surfaces: ResMut<WindowSurfaces>,
    mut events: EventReader<WindowDestroyed>,
) {
    for event in events.iter() {
        surfaces.windows.remove(&event.window);
    }
}

#[derive(Resource)]
struct WindowQuery(QueryState<&'static WindowState>);

pub fn render_surfaces(
    // mut surfaces: ResMut<WindowSurfaces>,
    // windows: Query<&WindowState>,
    // device: Res<RenderDevice>,
    // queue: Res<RenderQueue>,
    // render_graph: Res<RenderGraph>,
    world: &mut World,
) {
    world.resource_scope::<RenderGraph, ()>(|world, mut render_graph| {
        for node in &mut render_graph.nodes {
            node.update(world);
        }
    });

    world.resource_scope::<WindowSurfaces, ()>(|world, mut surfaces| {
        world.resource_scope::<WindowQuery, ()>(|world, mut windows| {
            let device = world.resource::<RenderDevice>();
            let queue = world.resource::<RenderQueue>();
            let render_graph = world.resource::<RenderGraph>();

            for (entity, surface) in surfaces.windows.iter_mut() {
                let output = match surface.surface.get_current_texture() {
                    Ok(output) => output,
                    Err(err) => {
                        let size = windows.0.get(&world, *entity).unwrap().0.inner_size();
                        surface.config.width = size.width;
                        surface.config.height = size.height;

                        match err {
                            SurfaceError::Outdated => {
                                surface.surface.configure(&device.0, &surface.config);
                            }
                            SurfaceError::Lost => {
                                surface.surface.configure(&device.0, &surface.config);
                            }
                            SurfaceError::OutOfMemory => {
                                tracing::error!("OOM");
                                std::process::exit(1);
                            }
                            _ => {
                                tracing::error!("failed to get window surface: {}", err);
                            }
                        }

                        continue;
                    }
                };

                let view = output
                    .texture
                    .create_view(&TextureViewDescriptor::default());

                let mut encoder = device.0.create_command_encoder(&CommandEncoderDescriptor {
                    label: Some("render_encoder"),
                });

                {
                    let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                        label: Some("render_pass"),
                        color_attachments: &[Some(RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: Operations {
                                load: LoadOp::Clear(Color {
                                    r: 0.1,
                                    g: 0.2,
                                    b: 0.3,
                                    a: 1.0,
                                }),
                                store: true,
                            },
                        })],
                        depth_stencil_attachment: None,
                    });
                }

                let mut ctx = RenderContext {
                    encoder: &mut encoder,
                    view: &view,
                    device: &device.0,
                };

                for node in &render_graph.nodes {
                    node.render(world, &mut ctx);
                }

                queue.0.submit(std::iter::once(encoder.finish()));
                output.present();
            }
        });
    });
}
