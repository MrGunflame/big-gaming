pub mod buffer;
pub mod events;
pub mod image;
pub mod layout;
pub mod style;
pub mod text;
pub mod ui;
pub mod window;

use bytemuck::{Pod, Zeroable};
use glam::Vec2;
use layout::{Container, Element, Rect};
use tracing::instrument::WithSubscriber;
use ui::{BuildPrimitiveElement, RenderContext, UiPass, UiPipeline};
use wgpu::util::DeviceExt;
use wgpu::{
    BindGroup, BufferAddress, BufferUsages, IndexFormat, PipelineLayout, RenderPipeline,
    VertexAttribute, VertexBufferLayout, VertexFormat, VertexStepMode,
};
use winit::event::WindowEvent;
use winit::window::Window;

use crate::layout::Frame;
use crate::text::Text;

pub struct State {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    window: Window,
    pipelines: Vec<RenderPipeline>,
    pipeline_layouts: Vec<PipelineLayout>,
    ui_pass: UiPass,
    ui_pipeline: UiPipeline,
    frame: Frame,
}

impl State {
    async fn new(window: Window) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            dx12_shader_compiler: Default::default(),
        });

        let surface = unsafe { instance.create_surface(&window) }.unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (mut device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::default(),
                    label: None,
                },
                None,
            )
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);

        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .filter(|f| f.is_srgb())
            .next()
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };

        surface.configure(&device, &config);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let mut pipeline_layouts = vec![];
        let mut pipelines = vec![];

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("main_pipeline"),
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("main_pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        pipeline_layouts.push(render_pipeline_layout);
        pipelines.push(render_pipeline);

        let rect = Rect {
            position: Vec2::new(0.0, 0.0),
            width: size.width as f32 / 4.0,
            height: size.height as f32 / 4.0,
            // width: 100.0,
            // height: 100.0,
        };

        let mut frame = Frame::new(Vec2::new(size.width as f32, size.height as f32));
        frame.push(
            None,
            Element::Text(Text {
                position: Vec2::splat(0.0),
                text: "Hello World!\nNewline\nNL2".to_owned(),
                size: 45.0,
            }),
        );
        frame.push(
            None,
            Element::Image(crate::image::Image {
                position: Vec2::splat(0.0),
                image: ::image::io::Reader::open("img.png")
                    .unwrap()
                    .decode()
                    .unwrap()
                    .to_rgba8(),
                dimensions: Vec2::new(64.0, 64.0),
            }),
        );
        let container = frame.push(
            None,
            Element::Container(Container {
                position: Vec2::splat(0.0),
            }),
        );

        frame.push(
            Some(container),
            Element::Text(Text {
                position: Vec2::splat(0.0),
                text: "Im in a container".to_owned(),
                size: 20.0,
            }),
        );

        let ui_pipeline = UiPipeline::new(&device);

        let ui_pass = UiPass::new();

        Self {
            window,
            surface,
            device,
            queue,
            config,
            size,
            pipeline_layouts,
            pipelines,
            ui_pass,
            ui_pipeline,
            frame,
        }
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }

        self.frame
            .resize(Vec2::new(new_size.width as f32, new_size.height as f32));
    }

    pub fn input(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::MouseInput {
                device_id,
                state,
                button,
                modifiers,
            } => {}
            WindowEvent::CursorMoved {
                device_id,
                position,
                modifiers,
            } => {
                for (elem, layout) in self.frame.elements().zip(self.frame.layouts()) {
                    let rect = crate::events::Rect {
                        min: Vec2 {
                            x: layout.position.x,
                            y: layout.position.y,
                        },
                        max: Vec2 {
                            x: layout.position.x + layout.width,
                            y: layout.position.y + layout.height,
                        },
                    };

                    let cursor = Vec2::new(position.x as f32, position.y as f32);
                    dbg!(crate::events::hit_test(rect, cursor));
                }
            }
            _ => todo!(),
        }

        false
    }

    pub fn update(&mut self) {
        self.ui_pass.update(
            &self.ui_pipeline,
            &self.device,
            &self.queue,
            Vec2::new(self.size.width as f32, self.size.height as f32),
            &mut self.frame,
        );
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("render_encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
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

        let ctx = RenderContext {
            encoder: &mut encoder,
            view: &view,
            device: &self.device,
        };
        self.ui_pass.render(&self.ui_pipeline, ctx);

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

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
