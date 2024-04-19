use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use game_render::camera::{Camera, CameraUniform};
use game_render::graph::{Node, RenderContext};
use game_tracing::trace_span;
use parking_lot::{Mutex, RwLock};
use wgpu::util::{BufferInitDescriptor, DeviceExt, DrawIndirectArgs};
use wgpu::{
    BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, BlendState, BufferBindingType, BufferUsages,
    ColorTargetState, ColorWrites, Device, Face, FragmentState, FrontFace, LoadOp,
    MultisampleState, Operations, PipelineLayoutDescriptor, PolygonMode, PrimitiveState,
    PrimitiveTopology, RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline,
    RenderPipelineDescriptor, ShaderModuleDescriptor, ShaderSource, ShaderStages, StoreOp,
    TextureFormat, VertexState,
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
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
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
                buffers: &[],
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
    vertex_buffer: Mutex<Vec<Vertex>>,
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
            camera,
            vertex_buffer: Mutex::new(Vec::new()),
        }
    }

    fn update_buffers(&self) {
        let cmds = self.elements.read();

        let mut vertex_buffer = self.vertex_buffer.lock();
        vertex_buffer.clear();

        for cmd in &*cmds {
            vertex_buffer.push(Vertex {
                position: cmd.start.to_array(),
                color: cmd.color.as_rgba(),
                _pad0: 0,
            });
            vertex_buffer.push(Vertex {
                position: cmd.end.to_array(),
                color: cmd.color.as_rgba(),
                _pad0: 0,
            });
        }
    }
}

impl Node for GizmoPass {
    fn render(&self, ctx: &mut RenderContext<'_>) {
        let _span = trace_span!("GizmoPass::render").entered();

        let Some(camera) = self.camera.lock().clone() else {
            return;
        };

        self.update_buffers();
        let vertex_buffer = self.vertex_buffer.lock();

        // Don't start a render pass with 0 vertices, this will cause problems
        // because the vertex SSBO must contain at least one element.
        if vertex_buffer.len() == 0 {
            return;
        }

        let camera_buffer = ctx.device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&[CameraUniform::new(
                camera.transform,
                camera.projection,
            )]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let vertices = ctx.device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&vertex_buffer),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        });

        let bg = ctx.device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &self.pipeline.bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: vertices.as_entire_binding(),
                },
            ],
        });

        let indirect_buffer = ctx.device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: DrawIndirectArgs {
                vertex_count: 2,
                instance_count: (vertex_buffer.len() / 2) as u32,
                first_vertex: 0,
                first_instance: 0,
            }
            .as_bytes(),
            usage: BufferUsages::INDIRECT,
        });

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

        render_pass.set_bind_group(0, &bg, &[]);
        render_pass.draw_indirect(&indirect_buffer, 0);
    }
}

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
struct Vertex {
    position: [f32; 3],
    _pad0: u32,
    color: [f32; 4],
}
