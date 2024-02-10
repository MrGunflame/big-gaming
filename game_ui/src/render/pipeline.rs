use std::collections::HashMap;
use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use game_render::camera::RenderTarget;
use game_render::graph::{Node, RenderContext};
use game_tracing::trace_span;
use glam::UVec2;
use parking_lot::{Mutex, RwLock};
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

use super::remap::remap;
use super::DrawCommand;

const UI_SHADER: &str = include_str!("../../shaders/ui.wgsl");

#[derive(Debug)]
struct UiPipeline {
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

#[derive(Debug)]
pub struct UiPass {
    pipeline: UiPipeline,
    elements: Arc<RwLock<HashMap<RenderTarget, Vec<DrawCommand>>>>,
    gpu_elements: Mutex<HashMap<RenderTarget, Vec<GpuElement>>>,
}

impl UiPass {
    pub(super) fn new(
        device: &Device,
        elems: Arc<RwLock<HashMap<RenderTarget, Vec<DrawCommand>>>>,
    ) -> Self {
        Self {
            pipeline: UiPipeline::new(device),
            elements: elems,
            gpu_elements: Mutex::new(HashMap::new()),
        }
    }

    fn update_buffers(
        &self,
        target: RenderTarget,
        device: &Device,
        queue: &Queue,
        viewport_size: UVec2,
    ) {
        let mut gpu_elements = self.gpu_elements.lock();
        gpu_elements.clear();

        let draw_cmds = self.elements.read();
        let Some(cmds) = draw_cmds.get(&target) else {
            gpu_elements.insert(target, Vec::new());
            return;
        };

        let mut elems = Vec::new();
        for cmd in cmds {
            let elem = GpuElement::new(device, queue, &self.pipeline, cmd, viewport_size);
            elems.push(elem);
        }

        gpu_elements.insert(target, elems);
    }
}

impl Node for UiPass {
    fn render(&self, ctx: &mut RenderContext<'_>) {
        let _span = trace_span!("UiPass::render").entered();

        self.update_buffers(ctx.render_target, ctx.device, ctx.queue, ctx.size);
        let elements = self.gpu_elements.lock();
        let elements = elements.get(&ctx.render_target).unwrap();

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

#[derive(Debug)]
struct GpuElement {
    vertices: Buffer,
    indices: Buffer,
    num_vertices: u32,
    bind_group: BindGroup,
}

impl GpuElement {
    fn new(
        device: &Device,
        queue: &Queue,
        pipeline: &UiPipeline,
        cmd: &DrawCommand,
        viewport_size: UVec2,
    ) -> Self {
        let _span = trace_span!("GpuElement::new").entered();

        if cfg!(debug_assertions) && (cmd.image.height() == 0 || cmd.image.width() == 0) {
            panic!(
                "attempted to render a image with zero dimension x={}, y={}",
                cmd.image.width(),
                cmd.image.height()
            );
        }

        let min = remap(cmd.position.min.as_vec2(), viewport_size.as_vec2());
        let max = remap(cmd.position.max.as_vec2(), viewport_size.as_vec2());

        let vertices = [
            Vertex {
                position: [min.x, min.y, 0.0],
                uv: [0.0, 0.0],
                color: cmd.color.as_rgba(),
            },
            Vertex {
                position: [min.x, max.y, 0.0],
                uv: [0.0, 1.0],
                color: cmd.color.as_rgba(),
            },
            Vertex {
                position: [max.x, max.y, 0.0],
                uv: [1.0, 1.0],
                color: cmd.color.as_rgba(),
            },
            Vertex {
                position: [max.x, min.y, 0.0],
                uv: [1.0, 0.0],
                color: cmd.color.as_rgba(),
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
                width: cmd.image.width(),
                height: cmd.image.height(),
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
            &cmd.image,
            ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * cmd.image.width()),
                rows_per_image: Some(cmd.image.height()),
            },
            Extent3d {
                width: cmd.image.width(),
                height: cmd.image.height(),
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
