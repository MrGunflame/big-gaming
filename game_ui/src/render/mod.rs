pub(crate) mod container;
pub(crate) mod debug;
pub mod image;
pub mod remap;
pub(crate) mod text;

use std::collections::HashMap;
use std::sync::Arc;

use ::image::{ImageBuffer, Rgba};
use bytemuck::{Pod, Zeroable};
use game_render::camera::RenderTarget;
use game_render::graph::{Node, RenderContext};
use game_render::Renderer;
use game_tracing::trace_span;
use game_window::windows::WindowId;
use glam::{UVec2, Vec2};
use parking_lot::RwLock;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{
    AddressMode, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType, BlendState,
    Buffer, BufferAddress, BufferUsages, ColorTargetState, ColorWrites, Device, Extent3d, Face,
    FilterMode, FragmentState, FrontFace, ImageCopyTexture, ImageDataLayout, IndexFormat, LoadOp,
    MultisampleState, Operations, Origin3d, PipelineLayoutDescriptor, PolygonMode, PrimitiveState,
    PrimitiveTopology, Queue, RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline,
    RenderPipelineDescriptor, Sampler, SamplerBindingType, SamplerDescriptor,
    ShaderModuleDescriptor, ShaderSource, ShaderStages, StoreOp, TextureAspect, TextureDescriptor,
    TextureDimension, TextureFormat, TextureSampleType, TextureUsages, TextureViewDescriptor,
    TextureViewDimension, VertexAttribute, VertexBufferLayout, VertexFormat, VertexState,
    VertexStepMode,
};

use crate::layout::computed_style::ComputedStyle;
use crate::layout::LayoutTree;

pub use self::image::Image;
pub use self::text::Text;
pub use crate::layout::{Element, ElementBody};

const UI_SHADER: &str = include_str!("../../shaders/ui.wgsl");

pub struct UiRenderer {
    pipeline: Arc<UiPipeline>,
    windows: HashMap<WindowId, LayoutTree>,
    elements: Arc<RwLock<HashMap<WindowId, Vec<PrimitiveElement>>>>,
}

impl UiRenderer {
    pub fn new(renderer: &Renderer) -> Self {
        let device = renderer.device();

        let pipeline = Arc::new(UiPipeline::new(device));
        let elements = Arc::new(RwLock::new(HashMap::new()));

        renderer.add_to_graph(UiPass {
            pipeline: pipeline.clone(),
            elements: elements.clone(),
        });

        Self {
            pipeline,
            windows: HashMap::new(),
            elements,
        }
    }

    pub fn insert(&mut self, id: WindowId, size: UVec2) {
        self.windows.insert(id, LayoutTree::new());
        self.resize(id, size);

        let mut elems = self.elements.write();
        elems.insert(id, vec![]);
    }

    pub fn get_mut(&mut self, id: WindowId) -> Option<&mut LayoutTree> {
        self.windows.get_mut(&id)
    }

    pub fn remove(&mut self, id: WindowId) {
        self.windows.remove(&id);

        let mut elems = self.elements.write();
        elems.remove(&id);
    }

    pub fn resize(&mut self, id: WindowId, size: UVec2) {
        if let Some(tree) = self.windows.get_mut(&id) {
            tree.resize(size);
        }
    }

    pub fn update(&mut self, device: &Device, queue: &Queue) {
        let _span = trace_span!("UiRenderer::update").entered();

        for (id, tree) in self.windows.iter_mut() {
            if !tree.is_changed() {
                continue;
            }

            let size = tree.size();

            tree.compute_layout();

            let mut elems = vec![];
            for (elem, layout) in tree.elements().zip(tree.layouts()) {
                // Don't render elements with a zero size.
                if layout.width == 0 || layout.height == 0 {
                    continue;
                }

                // Don't render elements that start outside of the viewport.
                if layout.position.x > size.x || layout.position.y > size.y {
                    continue;
                }

                if let Some(elem) = elem.build(
                    &layout.style,
                    Rect {
                        min: layout.position,
                        max: layout.position + UVec2::new(layout.width, layout.height),
                    },
                    &self.pipeline,
                    device,
                    queue,
                    size,
                ) {
                    elems.push(elem);
                }
            }

            tree.unchanged();

            *self.elements.write().get_mut(id).unwrap() = elems;
        }
    }
}

#[derive(Debug)]
pub(crate) struct PrimitiveElement {
    /// Vertex buffer
    vertices: Buffer,
    /// Index buffer
    indices: Buffer,
    num_vertices: u32,
    bind_group: BindGroup,
}

impl PrimitiveElement {
    pub(crate) fn new(
        pipeline: &UiPipeline,
        device: &Device,
        queue: &Queue,
        min: Vec2,
        max: Vec2,
        image: &ImageBuffer<Rgba<u8>, Vec<u8>>,
        color: [f32; 4],
    ) -> Self {
        let _span = trace_span!("PrimitiveElement::new").entered();

        if cfg!(debug_assertions) && (image.height() == 0 || image.width() == 0) {
            panic!(
                "attempted to render a image with zero dimension x={}, y={}",
                image.width(),
                image.height()
            );
        }

        let vertices = [
            Vertex {
                position: [min.x, min.y, 0.0],
                uv: [0.0, 0.0],
                color,
            },
            Vertex {
                position: [min.x, max.y, 0.0],
                uv: [0.0, 1.0],
                color,
            },
            Vertex {
                position: [max.x, max.y, 0.0],
                uv: [1.0, 1.0],
                color,
            },
            Vertex {
                position: [max.x, min.y, 0.0],
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
            image,
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

pub(crate) trait BuildPrimitiveElement {
    fn build(
        &self,
        style: &ComputedStyle,
        layout: Rect,
        pipeline: &UiPipeline,
        device: &Device,
        queue: &Queue,
        size: UVec2,
    ) -> Option<PrimitiveElement>;
}

#[derive(Debug)]
pub(crate) struct UiPipeline {
    bind_group_layout: BindGroupLayout,
    pipeline: RenderPipeline,
    sampler: Sampler,
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
            source: ShaderSource::Wgsl(UI_SHADER.into()),
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
                    format: TextureFormat::Bgra8Unorm,
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
    pub min: UVec2,
    pub max: UVec2,
}

impl Rect {
    #[inline]
    pub fn size(self) -> UVec2 {
        self.max - self.min
    }

    #[inline]
    pub fn width(self) -> u32 {
        self.max.x - self.min.x
    }

    #[inline]
    pub fn height(self) -> u32 {
        self.max.y - self.min.y
    }
}

#[derive(Debug)]
pub struct UiPass {
    pipeline: Arc<UiPipeline>,
    elements: Arc<RwLock<HashMap<WindowId, Vec<PrimitiveElement>>>>,
}

impl Node for UiPass {
    fn render(&self, ctx: &mut RenderContext<'_>) {
        let _span = trace_span!("UiPass::render").entered();

        let elems = self.elements.read();
        // TODO: Rendering to non-window targets.
        let RenderTarget::Window(window) = ctx.render_target else {
            return;
        };

        let Some(elements) = elems.get(&window) else {
            return;
        };

        let mut render_pass = ctx.encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("ui_pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: ctx.target,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Load,
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        render_pass.set_pipeline(&self.pipeline.pipeline);

        for elem in elements {
            render_pass.set_bind_group(0, &elem.bind_group, &[]);
            render_pass.set_vertex_buffer(0, elem.vertices.slice(..));
            render_pass.set_index_buffer(elem.indices.slice(..), IndexFormat::Uint32);

            render_pass.draw_indexed(0..elem.num_vertices, 0, 0..1);
        }
    }
}
