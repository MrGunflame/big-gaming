use std::num::NonZeroU32;
use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use game_render::api::{
    BindingResource, BufferInitDescriptor, CommandQueue, DescriptorSetDescriptor,
    DescriptorSetEntry, DescriptorSetLayout, DescriptorSetLayoutDescriptor, Pipeline,
    PipelineDescriptor, RenderPassColorAttachment, RenderPassDescriptor, Texture,
    TextureViewDescriptor,
};
use game_render::backend::allocator::UsageFlags;
use game_render::backend::{
    BufferUsage, ColorTargetState, DescriptorBinding, DescriptorType, Face, FragmentStage,
    FrontFace, LoadOp, PipelineStage, PrimitiveTopology, ShaderStages, StoreOp, TextureFormat,
    VertexStage,
};
use game_render::camera::{Camera, CameraUniform};
use game_render::graph::{Node, RenderContext, SlotLabel};
use game_render::pipeline_cache::{PipelineBuilder, PipelineCache};
use game_render::shader::{Shader, ShaderConfig, ShaderLanguage, ShaderSource};
use game_tracing::trace_span;
use parking_lot::{Mutex, RwLock};

use super::DrawCommand;

const SHADER: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/shaders/line.wgsl");

pub struct GizmoPipeline {
    descriptor_set_layout: DescriptorSetLayout,
    pipeline: PipelineCache<GizmoPipelineBuilder>,
}

impl GizmoPipeline {
    pub fn new(queue: &CommandQueue<'_>) -> Self {
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

        let pipeline = PipelineCache::new(
            GizmoPipelineBuilder {
                descriptor_set_layout: bind_group_layout.clone(),
            },
            vec![ShaderConfig {
                source: ShaderSource::File(SHADER.into()),
                language: ShaderLanguage::Wgsl,
            }],
        );

        Self {
            descriptor_set_layout: bind_group_layout,
            pipeline,
        }
    }
}

#[derive(Debug)]
struct GizmoPipelineBuilder {
    descriptor_set_layout: DescriptorSetLayout,
}

impl PipelineBuilder for GizmoPipelineBuilder {
    fn build(
        &self,
        queue: &CommandQueue<'_>,
        shaders: &[Shader],
        format: TextureFormat,
    ) -> Pipeline {
        queue.create_pipeline(&PipelineDescriptor {
            topology: PrimitiveTopology::LineList,
            front_face: FrontFace::Ccw,
            cull_mode: Some(Face::Back),
            descriptors: &[&self.descriptor_set_layout],
            stages: &[
                PipelineStage::Vertex(VertexStage {
                    shader: &shaders[0],
                    entry: "vs_main",
                }),
                PipelineStage::Fragment(FragmentStage {
                    shader: &shaders[0],
                    entry: "fs_main",
                    targets: &[ColorTargetState {
                        format,
                        blend: None,
                    }],
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
        queue: &CommandQueue<'_>,
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

        let Some(camera) = &*self.camera.lock() else {
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
            flags: UsageFlags::empty(),
        });

        let vertices = ctx.queue.create_buffer_init(&BufferInitDescriptor {
            contents: bytemuck::cast_slice(&vertex_buffer),
            usage: BufferUsage::STORAGE,
            flags: UsageFlags::empty(),
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

        let surface_texture = ctx.read::<Texture>(SlotLabel::SURFACE).unwrap().clone();

        let render_pipeline = self
            .pipeline
            .pipeline
            .get(&mut ctx.queue, surface_texture.format());

        let mut render_pass = ctx.queue.run_render_pass(&RenderPassDescriptor {
            name: "Gizmos",
            color_attachments: &[RenderPassColorAttachment {
                target: &surface_texture.create_view(&TextureViewDescriptor::default()),
                load_op: LoadOp::Load,
                store_op: StoreOp::Store,
            }],
            depth_stencil_attachment: None,
        });

        render_pass.set_pipeline(&render_pipeline);

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
