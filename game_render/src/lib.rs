pub mod buffer;
pub mod events;
pub mod image;
pub mod layout;
pub mod style;
pub mod text;
pub mod ui;
// pub mod window;

use std::collections::HashMap;
use std::sync::Arc;

use bevy_app::{App, Plugin};
use bevy_ecs::prelude::{Component, Entity, EventReader};
use bevy_ecs::system::{Query, Res, ResMut, Resource};
use bytemuck::{Pod, Zeroable};
use events::Event;
use game_window::events::{WindowCreated, WindowResized};
use game_window::{Window, WindowState};
use glam::Vec2;
use layout::{Container, Element, Key, Rect};
use tracing::Instrument;
use ui::{RenderContext, UiPass, UiPipeline};
use wgpu::{
    Adapter, Backends, BufferAddress, Color, CommandEncoder, CommandEncoderDescriptor, Device,
    DeviceDescriptor, Features, IndexFormat, Instance, InstanceDescriptor, Limits, LoadOp,
    Operations, PipelineLayout, PowerPreference, Queue, RenderPassColorAttachment,
    RenderPassDescriptor, RenderPipeline, RequestAdapterOptions, Surface, SurfaceConfiguration,
    SurfaceError, TextureFormat, TextureUsages, TextureViewDescriptor, VertexAttribute,
    VertexBufferLayout, VertexFormat, VertexStepMode,
};
use winit::dpi::PhysicalSize;
use winit::event::WindowEvent;

use crate::layout::Frame;
use crate::text::Text;

#[derive(Copy, Clone, Debug, Default)]
pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
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

        app.add_system(create_surfaces);
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

pub fn render_surfaces(
    mut surfaces: ResMut<WindowSurfaces>,
    windows: Query<&WindowState>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
) {
    for (entity, surface) in surfaces.windows.iter_mut() {
        let output = match surface.surface.get_current_texture() {
            Ok(output) => output,
            Err(err) => {
                let size = windows.get(*entity).unwrap().0.inner_size();
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

        queue.0.submit(std::iter::once(encoder.finish()));
        output.present();
    }
}

// pub struct State {
//     surface: wgpu::Surface,
//     device: wgpu::Device,
//     queue: wgpu::Queue,
//     config: wgpu::SurfaceConfiguration,
//     size: winit::dpi::PhysicalSize<u32>,
//     window: Window,
//     pipelines: Vec<RenderPipeline>,
//     pipeline_layouts: Vec<PipelineLayout>,
//     ui_pass: UiPass,
//     ui_pipeline: UiPipeline,
//     frame: Frame,
//     events: Vec<Event>,
//     active_states: HashMap<Key, crate::events::State>,
// }

// impl State {
//     async fn new(window: Window) -> Self {
//         let size = window.inner_size();

//         let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
//             backends: wgpu::Backends::all(),
//             dx12_shader_compiler: Default::default(),
//         });

//         let surface = unsafe { instance.create_surface(&window) }.unwrap();

//         let adapter = instance
//             .request_adapter(&wgpu::RequestAdapterOptions {
//                 power_preference: wgpu::PowerPreference::default(),
//                 // compatible_surface: Some(&surface),
//                 compatible_surface: None,
//                 force_fallback_adapter: false,
//             })
//             .await
//             .unwrap();

//         let (mut device, queue) = adapter
//             .request_device(
//                 &wgpu::DeviceDescriptor {
//                     features: wgpu::Features::empty(),
//                     limits: wgpu::Limits::default(),
//                     label: None,
//                 },
//                 None,
//             )
//             .await
//             .unwrap();

//         let surface_caps = surface.get_capabilities(&adapter);

//         let surface_format = surface_caps
//             .formats
//             .iter()
//             .copied()
//             .filter(|f| f.is_srgb())
//             .next()
//             .unwrap_or(surface_caps.formats[0]);

//         let config = wgpu::SurfaceConfiguration {
//             usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
//             format: surface_format,
//             width: size.width,
//             height: size.height,
//             present_mode: surface_caps.present_modes[0],
//             alpha_mode: surface_caps.alpha_modes[0],
//             view_formats: vec![],
//         };

//         surface.configure(&device, &config);

//         let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
//             label: Some("shader"),
//             source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
//         });

//         let mut pipeline_layouts = vec![];
//         let mut pipelines = vec![];

//         let render_pipeline_layout =
//             device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
//                 label: Some("main_pipeline"),
//                 bind_group_layouts: &[],
//                 push_constant_ranges: &[],
//             });

//         let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
//             label: Some("main_pipeline"),
//             layout: Some(&render_pipeline_layout),
//             vertex: wgpu::VertexState {
//                 module: &shader,
//                 entry_point: "vs_main",
//                 buffers: &[Vertex::desc()],
//             },
//             fragment: Some(wgpu::FragmentState {
//                 module: &shader,
//                 entry_point: "fs_main",
//                 targets: &[Some(wgpu::ColorTargetState {
//                     format: config.format,
//                     blend: Some(wgpu::BlendState::REPLACE),
//                     write_mask: wgpu::ColorWrites::ALL,
//                 })],
//             }),
//             primitive: wgpu::PrimitiveState {
//                 topology: wgpu::PrimitiveTopology::TriangleList,
//                 strip_index_format: None,
//                 front_face: wgpu::FrontFace::Ccw,
//                 cull_mode: Some(wgpu::Face::Back),
//                 polygon_mode: wgpu::PolygonMode::Fill,
//                 unclipped_depth: false,
//                 conservative: false,
//             },
//             depth_stencil: None,
//             multisample: wgpu::MultisampleState {
//                 count: 1,
//                 mask: !0,
//                 alpha_to_coverage_enabled: false,
//             },
//             multiview: None,
//         });

//         pipeline_layouts.push(render_pipeline_layout);
//         pipelines.push(render_pipeline);

//         let rect = Rect {
//             position: Vec2::new(0.0, 0.0),
//             width: size.width as f32 / 4.0,
//             height: size.height as f32 / 4.0,
//             // width: 100.0,
//             // height: 100.0,
//         };

//         let mut frame = Frame::new(Vec2::new(size.width as f32, size.height as f32));
//         frame.push(
//             None,
//             Element::Text(Text {
//                 position: Vec2::splat(0.0),
//                 text: "Hello World!\nNewline\nNL2".to_owned(),
//                 size: 45.0,
//             }),
//         );
//         frame.push(
//             None,
//             Element::Image(crate::image::Image {
//                 position: Vec2::splat(0.0),
//                 image: ::image::io::Reader::open("img.png")
//                     .unwrap()
//                     .decode()
//                     .unwrap()
//                     .to_rgba8(),
//                 dimensions: Vec2::new(64.0, 64.0),
//             }),
//         );
//         let container = frame.push(
//             None,
//             Element::Container(Container {
//                 position: Vec2::splat(0.0),
//             }),
//         );

//         frame.push(
//             Some(container),
//             Element::Text(Text {
//                 position: Vec2::splat(0.0),
//                 text: "Im in a container".to_owned(),
//                 size: 20.0,
//             }),
//         );

//         let ui_pipeline = UiPipeline::new(&device);

//         let ui_pass = UiPass::new();

//         Self {
//             window,
//             surface,
//             device,
//             queue,
//             config,
//             size,
//             pipeline_layouts,
//             pipelines,
//             ui_pass,
//             ui_pipeline,
//             frame,
//             events: vec![],
//             active_states: HashMap::new(),
//         }
//     }

//     pub fn window(&self) -> &Window {
//         &self.window
//     }

//     pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
//         if new_size.width > 0 && new_size.height > 0 {
//             self.size = new_size;
//             self.config.width = new_size.width;
//             self.config.height = new_size.height;
//             self.surface.configure(&self.device, &self.config);
//         }

//         self.frame
//             .resize(Vec2::new(new_size.width as f32, new_size.height as f32));
//     }

//     pub fn input(&mut self, event: &WindowEvent) -> bool {
//         match event {
//             WindowEvent::MouseInput {
//                 device_id,
//                 state,
//                 button,
//                 modifiers,
//             } => {}
//             WindowEvent::CursorMoved {
//                 device_id,
//                 position,
//                 modifiers,
//             } => {
//                 for ((elem, layout), key) in self
//                     .frame
//                     .elements()
//                     .zip(self.frame.layouts())
//                     .zip(self.frame.keys())
//                 {
//                     let rect = crate::events::Rect {
//                         min: Vec2 {
//                             x: layout.position.x,
//                             y: layout.position.y,
//                         },
//                         max: Vec2 {
//                             x: layout.position.x + layout.width,
//                             y: layout.position.y + layout.height,
//                         },
//                     };

//                     let cursor = Vec2::new(position.x as f32, position.y as f32);

//                     let hits = crate::events::hit_test(rect, cursor);
//                     let state = if hits {
//                         crate::events::State::Hovered
//                     } else {
//                         crate::events::State::None
//                     };

//                     let cell = self
//                         .active_states
//                         .entry(key)
//                         .or_insert(crate::events::State::None);

//                     if *cell != state {
//                         *cell = state;
//                         self.events.push(Event { key, state });
//                         dbg!("new");
//                     }
//                 }
//             }
//             _ => todo!(),
//         }

//         false
//     }

//     pub fn update(&mut self) {
//         self.ui_pass.update(
//             &self.ui_pipeline,
//             &self.device,
//             &self.queue,
//             Vec2::new(self.size.width as f32, self.size.height as f32),
//             &mut self.frame,
//         );
//     }

//     pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
//         let output = self.surface.get_current_texture()?;

//         let view = output
//             .texture
//             .create_view(&wgpu::TextureViewDescriptor::default());

//         let mut encoder = self
//             .device
//             .create_command_encoder(&wgpu::CommandEncoderDescriptor {
//                 label: Some("render_encoder"),
//             });

//         {
//             let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
//                 label: Some("render_pass"),
//                 color_attachments: &[Some(wgpu::RenderPassColorAttachment {
//                     view: &view,
//                     resolve_target: None,
//                     ops: wgpu::Operations {
//                         load: wgpu::LoadOp::Clear(wgpu::Color {
//                             r: 0.1,
//                             g: 0.2,
//                             b: 0.3,
//                             a: 1.0,
//                         }),
//                         store: true,
//                     },
//                 })],
//                 depth_stencil_attachment: None,
//             });
//         }

//         let ctx = RenderContext {
//             encoder: &mut encoder,
//             view: &view,
//             device: &self.device,
//         };
//         self.ui_pass.render(&self.ui_pipeline, ctx);

//         self.queue.submit(std::iter::once(encoder.finish()));
//         output.present();

//         Ok(())
//     }
// }

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
pub struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
}

const VERTICES: &[Vertex] = &[
    Vertex {
        position: [-0.0868241, 0.49240386, 0.0],
        color: [0.5, 0.0, 0.5],
    }, // A
    Vertex {
        position: [-0.49513406, 0.06958647, 0.0],
        color: [0.5, 0.0, 0.5],
    }, // B
    Vertex {
        position: [-0.21918549, -0.44939706, 0.0],
        color: [0.5, 0.0, 0.5],
    }, // C
    Vertex {
        position: [0.35966998, -0.3473291, 0.0],
        color: [0.5, 0.0, 0.5],
    }, // D
    Vertex {
        position: [0.44147372, 0.2347359, 0.0],
        color: [0.5, 0.0, 0.5],
    }, // E
];

const INDICES: &[u16] = &[0, 1, 4, 1, 2, 4, 2, 3, 4];

impl Vertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as BufferAddress,
            step_mode: VertexStepMode::Vertex,
            attributes: &[
                VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: VertexFormat::Float32x3,
                },
                VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as BufferAddress,
                    shader_location: 1,
                    format: VertexFormat::Float32x3,
                },
            ],
        }
    }
}
