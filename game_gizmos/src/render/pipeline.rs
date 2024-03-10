use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use game_render::camera::{Camera, CameraUniform};
use game_render::graph::{Node, RenderContext};
use game_tracing::trace_span;
use parking_lot::{Mutex, RwLock};
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{
    BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, BlendState, Buffer, BufferAddress, BufferBindingType,
    BufferUsages, ColorTargetState, ColorWrites, Device, Face, FragmentState, FrontFace, LoadOp,
    MultisampleState, Operations, PipelineLayoutDescriptor, PolygonMode, PrimitiveState,
    PrimitiveTopology, Queue, RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline,
    RenderPipelineDescriptor, ShaderModuleDescriptor, ShaderSource, ShaderStages, StoreOp,
    TextureFormat, VertexAttribute, VertexBufferLayout, VertexFormat, VertexState, VertexStepMode,
};

use super::DrawCommand;

const SHADER: &str = include_str!("../../shaders/line.wgsl");

pub struct GizmoPipeline {
    pipeline: RenderPipeline,
    bind_group_layout: BindGroupLayout,
}

impl GizmoPipeline {
    pub fn new(device: &Device) -> Self {
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("gizmo_layout"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("gizmo_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("gizmo_shader"),
            source: ShaderSource::Wgsl(SHADER.into()),
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("gizmo_pipeline"),
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
                topology: PrimitiveTopology::LineList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                conservative: false,
                polygon_mode: PolygonMode::Fill,
                unclipped_depth: false,
            },
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview: None,
        });

        Self {
            bind_group_layout,
            pipeline,
        }
    }
}

pub struct GizmoPass {
    pipeline: GizmoPipeline,
    camera: Arc<Mutex<Option<Camera>>>,
    elements: Arc<RwLock<Vec<DrawCommand>>>,
    gpu_elements: Mutex<Vec<GpuElement>>,
}

impl GizmoPass {
    pub(crate) fn new(
        device: &Device,
        elements: Arc<RwLock<Vec<DrawCommand>>>,
        camera: Arc<Mutex<Option<Camera>>>,
    ) -> Self {
        Self {
            pipeline: GizmoPipeline::new(device),
            elements,
            gpu_elements: Mutex::new(Vec::new()),
            camera,
        }
    }

    fn update_buffers(&self, device: &Device) {
        let mut gpu_elements = self.gpu_elements.lock();
        gpu_elements.clear();

        let cmds = self.elements.read();

        let mut elems = Vec::new();
        for cmd in &*cmds {
            let elem = GpuElement::new(device, cmd);
            elems.push(elem);
        }

        *gpu_elements = elems;
    }
}

impl Node for GizmoPass {
    fn render(&self, ctx: &mut RenderContext<'_>) {
        let _span = trace_span!("GizmoPass::render").entered();

        let Some(camera) = self.camera.lock().clone() else {
            return;
        };

        let camera_buffer = ctx.device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&[CameraUniform::new(
                camera.transform,
                camera.projection,
            )]),
            usage: BufferUsages::VERTEX | BufferUsages::UNIFORM,
        });

        let bg = ctx.device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &self.pipeline.bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        self.update_buffers(ctx.device);
        let elements = self.gpu_elements.lock();

        let mut render_pass = ctx.encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("gizmo_pass"),
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

        for elem in &*elements {
            render_pass.set_bind_group(0, &bg, &[]);
            render_pass.set_vertex_buffer(0, elem.vertices.slice(..));

            render_pass.draw(0..elem.num_vertices, 0..1);
        }
    }
}

struct GpuElement {
    vertices: Buffer,
    num_vertices: u32,
}

impl GpuElement {
    fn new(device: &Device, cmd: &DrawCommand) -> Self {
        let vertices = [
            Vertex {
                position: cmd.start.to_array(),
                color: cmd.color.as_rgba(),
            },
            Vertex {
                position: cmd.end.to_array(),
                color: cmd.color.as_rgba(),
            },
        ];
        let num_vertices = 2;

        let vertices = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&vertices),
            usage: BufferUsages::VERTEX,
        });

        Self {
            vertices,
            num_vertices,
        }
    }
}

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
struct Vertex {
    position: [f32; 3],
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
                    format: VertexFormat::Float32x4,
                },
            ],
        }
    }
}
