use std::collections::HashMap;
use std::num::NonZeroU32;
use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use game_render::api::{
    BindingResource, BufferInitDescriptor, CommandQueue, DescriptorSetDescriptor,
    DescriptorSetEntry, DescriptorSetLayout, DescriptorSetLayoutDescriptor, Pipeline,
    PipelineDescriptor, RenderPassColorAttachment, RenderPassDescriptor, Texture,
};
use game_render::backend::{
    BufferUsage, DescriptorBinding, DescriptorType, Face, FragmentStage, FrontFace, LoadOp,
    PipelineStage, PrimitiveTopology, ShaderModule, ShaderSource, ShaderStages, StoreOp,
    TextureFormat, VertexStage,
};
use game_render::camera::{Camera, CameraUniform};
use game_render::graph::{Node, RenderContext, SlotLabel};
use game_tracing::trace_span;
use parking_lot::{Mutex, RwLock};

use super::DrawCommand;

const SHADER: &str = include_str!("../../shaders/line.wgsl");

pub struct GizmoPipeline {
    descriptor_set_layout: DescriptorSetLayout,
    pipelines: Mutex<HashMap<TextureFormat, Pipeline>>,
    shader: ShaderModule,
}

impl GizmoPipeline {
    pub fn new(queue: &mut CommandQueue<'_>) -> Self {
        let bind_group_layout =
            queue.create_descriptor_set_layout(&DescriptorSetLayoutDescriptor {
                bindings: &[
                    DescriptorBinding {
                        binding: 0,
                        visibility: ShaderStages::VERTEX,
                        kind: DescriptorType::Uniform,
                        count: NonZeroU32::MIN,
                    },
                    DescriptorBinding {
                        binding: 1,
                        visibility: ShaderStages::VERTEX,
                        kind: DescriptorType::Storage,
                        count: NonZeroU32::MIN,
                    },
                ],
            });

        let shader = queue.create_shader_module(ShaderSource::Wgsl(SHADER));

        Self {
            descriptor_set_layout: bind_group_layout,
            pipelines: Mutex::new(HashMap::new()),
            shader,
        }
    }

    fn build_pipeline(&self, format: TextureFormat, queue: &mut CommandQueue<'_>) -> Pipeline {
        queue.create_pipeline(&PipelineDescriptor {
            topology: PrimitiveTopology::LineList,
            front_face: FrontFace::Ccw,
            cull_mode: Some(Face::Back),
            descriptors: &[&self.descriptor_set_layout],
            stages: &[
                PipelineStage::Vertex(VertexStage {
                    shader: &self.shader,
                    entry: "vs_main",
                }),
                PipelineStage::Fragment(FragmentStage {
                    shader: &self.shader,
                    entry: "fs_main",
                    targets: &[format],
                }),
            ],
            depth_stencil_state: None,
            push_constant_ranges: &[],
        })
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
        queue: &mut CommandQueue<'_>,
        elements: Arc<RwLock<Vec<DrawCommand>>>,
        camera: Arc<Mutex<Option<Camera>>>,
    ) -> Self {
        Self {
            pipeline: GizmoPipeline::new(queue),
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
    fn render(&self, ctx: &mut RenderContext<'_, '_>) {
        let _span = trace_span!("GizmoPass::render").entered();

        let Some(camera) = *self.camera.lock() else {
            return;
        };

        self.update_buffers();
        let vertex_buffer = self.vertex_buffer.lock();

        // Don't start a render pass with 0 vertices, this will cause problems
        // because the vertex SSBO must contain at least one element.
        if vertex_buffer.len() == 0 {
            return;
        }

        let camera_buffer = ctx.queue.create_buffer_init(&BufferInitDescriptor {
            contents: bytemuck::cast_slice(&[CameraUniform::new(
                camera.transform,
                camera.projection,
            )]),
            usage: BufferUsage::UNIFORM,
        });

        let vertices = ctx.queue.create_buffer_init(&BufferInitDescriptor {
            contents: bytemuck::cast_slice(&vertex_buffer),
            usage: BufferUsage::STORAGE,
        });

        let bg = ctx.queue.create_descriptor_set(&DescriptorSetDescriptor {
            layout: &self.pipeline.descriptor_set_layout,
            entries: &[
                DescriptorSetEntry {
                    binding: 0,
                    resource: BindingResource::Buffer(&camera_buffer),
                },
                DescriptorSetEntry {
                    binding: 1,
                    resource: BindingResource::Buffer(&vertices),
                },
            ],
        });

        // let indirect_buffer = ctx.queue.create_buffer_init(&BufferInitDescriptor {
        //     contents: DrawIndirectArgs {
        //         vertex_count: 2,
        //         instance_count: (vertex_buffer.len() / 2) as u32,
        //         first_vertex: 0,
        //         first_instance: 0,
        //     }
        //     .as_bytes(),
        //     usage: BufferUsage::INDIRECT,
        // });

        let mut pipelines = self.pipeline.pipelines.lock();
        let render_pipeline = match pipelines.get(&ctx.format) {
            Some(pl) => pl,
            None => {
                let pl = self.pipeline.build_pipeline(ctx.format, ctx.queue);
                pipelines.insert(ctx.format, pl);
                pipelines.get(&ctx.format).unwrap()
            }
        };

        let surface_texture = ctx.read::<Texture>(SlotLabel::SURFACE).unwrap().clone();
        let mut render_pass = ctx.queue.run_render_pass(&RenderPassDescriptor {
            color_attachments: &[RenderPassColorAttachment {
                texture: &surface_texture,
                load_op: LoadOp::Load,
                store_op: StoreOp::Store,
            }],
            depth_stencil_attachment: None,
        });

        render_pass.set_pipeline(render_pipeline);

        render_pass.set_descriptor_set(0, &bg);
        // render_pass.draw_indirect(&indirect_buffer, 0);
        render_pass.draw(0..2, 0..(vertex_buffer.len() / 2) as u32);
    }
}

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
struct Vertex {
    position: [f32; 3],
    _pad0: u32,
    color: [f32; 4],
}
