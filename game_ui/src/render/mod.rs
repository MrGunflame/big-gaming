pub mod computed_style;
pub mod layout;
pub mod style;

mod container;
mod debug;
pub mod image;
mod systems;
mod text;

use std::collections::HashMap;

use ::image::{ImageBuffer, Rgba};
use bevy_app::{App, Plugin};
use bevy_ecs::prelude::Entity;
use bevy_ecs::system::Resource;
use bevy_ecs::world::{FromWorld, World};
use bytemuck::{Pod, Zeroable};
use game_render::graph::{Node, RenderContext, RenderGraph};
use game_render::{RenderDevice, RenderPlugin, RenderQueue};
use game_window::WindowState;
use glam::Vec2;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{
    AddressMode, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType, BlendState,
    Buffer, BufferAddress, BufferUsages, ColorTargetState, ColorWrites, Device, Extent3d, Face,
    FilterMode, FragmentState, FrontFace, ImageCopyTexture, ImageDataLayout, IndexFormat, LoadOp,
    MultisampleState, Operations, Origin3d, PipelineLayoutDescriptor, PolygonMode, PrimitiveState,
    PrimitiveTopology, Queue, RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline,
    RenderPipelineDescriptor, Sampler, SamplerBindingType, SamplerDescriptor,
    ShaderModuleDescriptor, ShaderSource, ShaderStages, TextureAspect, TextureDescriptor,
    TextureDimension, TextureFormat, TextureSampleType, TextureUsages, TextureViewDescriptor,
    TextureViewDimension, VertexAttribute, VertexBufferLayout, VertexFormat, VertexState,
    VertexStepMode,
};

use self::computed_style::{ComputedBounds, ComputedStyle};
use self::layout::LayoutTree;

pub use self::image::Image;
pub use self::layout::{Element, ElementBody};
use self::style::Style;
pub use self::text::Text;

pub struct RenderUiPlugin;

impl Plugin for RenderUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(RenderPlugin);

        app.init_resource::<UiPipeline>();

        let mut render_graph = app.world.resource_mut::<RenderGraph>();
        render_graph.push(UiPass::default());

        app.add_system(systems::layout_tree_size_window_creation);
        app.add_system(systems::layout_tree_window_resized);
    }
}

#[derive(Debug)]
struct PrimitiveElement {
    /// Vertex buffer
    vertices: Buffer,
    /// Index buffer
    indices: Buffer,
    num_vertices: u32,
    bind_group: BindGroup,
}

impl PrimitiveElement {
    fn new(
        pipeline: &UiPipeline,
        device: &Device,
        queue: &Queue,
        rect: Rect,
        image: &ImageBuffer<Rgba<u8>, Vec<u8>>,
        color: [f32; 4],
    ) -> Self {
        if cfg!(debug_assertions) {
            if image.height() == 0 || image.width() == 0 {
                panic!(
                    "attempted to render a image with zero dimension x={}, y={}",
                    image.width(),
                    image.height()
                );
            }
        }

        let vertices = [
            Vertex {
                position: [rect.min.x, rect.min.y, 0.0],
                uv: [0.0, 0.0],
                color,
            },
            Vertex {
                position: [rect.min.x, rect.max.y, 0.0],
                uv: [0.0, 1.0],
                color,
            },
            Vertex {
                position: [rect.max.x, rect.max.y, 0.0],
                uv: [1.0, 1.0],
                color,
            },
            Vertex {
                position: [rect.max.x, rect.min.y, 0.0],
                uv: [1.0, 0.0],
                color,
            },
        ];
        let indices: [u32; 6] = [0, 1, 2, 3, 0, 2];

        let num_vertices = indices.len() as u32;

        let vertices = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("primitive_element_vertex_buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: BufferUsages::VERTEX,
        });

        let indices = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("primitive_element_index_buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: BufferUsages::INDEX,
        });

        let texture = device.create_texture(&TextureDescriptor {
            label: Some("primitive_element_texture"),
            size: Extent3d {
                width: image.width(),
                height: image.height(),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8UnormSrgb,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            &image,
            ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * image.width()),
                rows_per_image: Some(image.height()),
            },
            Extent3d {
                width: image.width(),
                height: image.height(),
                depth_or_array_layers: 1,
            },
        );

        let texture_view = texture.create_view(&TextureViewDescriptor::default());

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("primitive_element_bind_group"),
            layout: &pipeline.bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&texture_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&pipeline.sampler),
                },
            ],
        });

        Self {
            vertices,
            indices,
            num_vertices,
            bind_group,
        }
    }
}

trait BuildPrimitiveElement {
    fn build(
        &self,
        style: &ComputedStyle,
        layout: Rect,
        pipeline: &UiPipeline,
        device: &Device,
        queue: &Queue,
        size: Vec2,
    ) -> Option<PrimitiveElement>;

    fn bounds(&self, style: &ComputedStyle) -> ComputedBounds;
}

#[derive(Debug, Resource)]
struct UiPipeline {
    bind_group_layout: BindGroupLayout,
    pipeline: RenderPipeline,
    sampler: Sampler,
}

impl FromWorld for UiPipeline {
    fn from_world(world: &mut World) -> Self {
        world.resource_scope::<RenderDevice, Self>(|_, device| Self::new(&device.0))
    }
}

impl UiPipeline {
    pub fn new(device: &Device) -> Self {
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("ui_layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("ui_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("ui_shader"),
            source: ShaderSource::Wgsl(include_str!("ui.wgsl").into()),
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("ui_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::layout()],
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(ColorTargetState {
                    format: TextureFormat::Bgra8UnormSrgb,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                polygon_mode: PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        let sampler = device.create_sampler(&SamplerDescriptor {
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Nearest,
            ..Default::default()
        });

        Self {
            bind_group_layout,
            pipeline,
            sampler,
        }
    }
}

/// A vertex in the UI.
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
struct Vertex {
    position: [f32; 3],
    uv: [f32; 2],
    color: [f32; 4],
}

impl Vertex {
    fn layout<'a>() -> VertexBufferLayout<'a> {
        VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as BufferAddress,
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
                    format: VertexFormat::Float32x2,
                },
                VertexAttribute {
                    offset: (std::mem::size_of::<[f32; 3]>() + std::mem::size_of::<[f32; 2]>())
                        as BufferAddress,
                    shader_location: 2,
                    format: VertexFormat::Float32x4,
                },
            ],
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Rect {
    pub min: Vec2,
    pub max: Vec2,
}

#[derive(Debug, Default)]
pub struct UiPass {
    elements: HashMap<Entity, Vec<PrimitiveElement>>,
}

impl Node for UiPass {
    fn update(&mut self, world: &mut World) {
        // Uh-oh
        world.resource_scope::<UiPipeline, ()>(|world, pipeline| {
            world.resource_scope::<RenderDevice, ()>(|world, device| {
                world.resource_scope::<RenderQueue, ()>(|world, queue| {
                    let mut query = world.query::<(Entity, &WindowState, &mut LayoutTree)>();

                    for (entity, window, mut frame) in query.iter_mut(world) {
                        if !frame.is_changed() {
                            continue;
                        }

                        let size = window.inner_size();

                        frame.compute_layout();

                        let mut elems = vec![];
                        for (elem, layout) in frame.elements().zip(frame.layouts()) {
                            // Don't render elements with a zero size.
                            if layout.width <= 0.0 || layout.height <= 0.0 {
                                continue;
                            }

                            // Don't render elements that start outside of the viewport.
                            if layout.position.x > size.width as f32
                                || layout.position.y > size.height as f32
                            {
                                continue;
                            }

                            if let Some(elem) = elem.build(
                                &layout.style,
                                Rect {
                                    min: layout.position,
                                    max: Vec2::new(
                                        layout.position.x + layout.width,
                                        layout.position.y + layout.height,
                                    ),
                                },
                                &pipeline,
                                &device.0,
                                &queue.0,
                                Vec2::new(size.width as f32, size.height as f32),
                            ) {
                                elems.push(elem);
                            }
                        }

                        frame.unchanged();

                        self.elements.insert(entity, elems);
                    }
                });
            });
        });
    }

    fn render(&self, world: &World, ctx: &mut RenderContext<'_>) {
        let pipeline = world.resource::<UiPipeline>();

        let Some(elements) = self.elements.get(&ctx.window) else {
            return;
        };

        let mut render_pass = ctx.encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("ui_pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &ctx.view,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Load,
                    store: true,
                },
            })],
            depth_stencil_attachment: None,
        });

        render_pass.set_pipeline(&pipeline.pipeline);

        for elem in elements {
            render_pass.set_bind_group(0, &elem.bind_group, &[]);
            render_pass.set_vertex_buffer(0, elem.vertices.slice(..));
            render_pass.set_index_buffer(elem.indices.slice(..), IndexFormat::Uint32);

            render_pass.draw_indexed(0..elem.num_vertices, 0, 0..1);
        }
    }
}
