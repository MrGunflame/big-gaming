use std::num::NonZeroU32;
use std::sync::Arc;

use game_render::api::{
    BindingResource, BufferDescriptor, CommandQueue, ComputePassDescriptor,
    DescriptorSetDescriptor, DescriptorSetEntry, DescriptorSetLayout,
    DescriptorSetLayoutDescriptor, Pipeline, PipelineDescriptor,
};
use game_render::backend::allocator::UsageFlags;
use game_render::backend::{
    BufferUsage, ComputeStage, DescriptorBinding, DescriptorType, DrawIndirectCommand, FrontFace,
    PipelineStage, PrimitiveTopology, PushConstantRange, ShaderStages, TextureFormat,
};
use game_render::graph::{Node, RenderContext};
use game_render::pipeline_cache::{PipelineBuilder, PipelineCache};
use game_render::shader::{Shader, ShaderConfig, ShaderLanguage, ShaderSource};
use game_tracing::trace_span;
use parking_lot::Mutex;

use crate::passes::{MeshStateImpl, State};

use super::{INDIRECT_DRAW_BUFFER, INSTANCE_BUFFER};

const SHADER: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/shaders/opaque_vertex/drawcall_gen.slang"
);

const WORKGROUP_SIZE: u32 = 64;

#[derive(Debug)]
pub struct DrawcallGenPass {
    state: Arc<Mutex<State>>,
    pipeline: PipelineCache<BuildPipeline>,
}

impl DrawcallGenPass {
    pub fn new(queue: &CommandQueue<'_>, state: Arc<Mutex<State>>) -> Self {
        let descriptor = queue.create_descriptor_set_layout(&DescriptorSetLayoutDescriptor {
            bindings: &[
                DescriptorBinding {
                    binding: 0,
                    visibility: ShaderStages::COMPUTE,
                    kind: DescriptorType::Storage,
                    count: NonZeroU32::MIN,
                },
                DescriptorBinding {
                    binding: 1,
                    visibility: ShaderStages::COMPUTE,
                    kind: DescriptorType::Storage,
                    count: NonZeroU32::MIN,
                },
                DescriptorBinding {
                    binding: 2,
                    visibility: ShaderStages::COMPUTE,
                    kind: DescriptorType::Storage,
                    count: NonZeroU32::MIN,
                },
            ],
        });

        let pipeline = PipelineCache::new(
            BuildPipeline { descriptor },
            vec![ShaderConfig {
                source: ShaderSource::File(SHADER.into()),
                language: ShaderLanguage::Slang,
            }],
        );

        Self { pipeline, state }
    }
}

impl Node for DrawcallGenPass {
    fn render<'a>(&self, ctx: &'a mut RenderContext<'_, 'a>) {
        let _span = trace_span!("DrawcallGenPass::render").entered();

        let mut state = self.state.lock();

        let MeshStateImpl::Vertex(mesh_state) = &mut state.mesh else {
            unreachable!()
        };

        if mesh_state.num_opauqe_instances == 0 {
            return;
        }

        let instances_in = mesh_state.opaque_instances.buffer(ctx.queue);

        let instances_out = ctx.queue.create_buffer(&BufferDescriptor {
            size: instances_in.size(),
            flags: UsageFlags::empty(),
            usage: BufferUsage::STORAGE,
        });

        let draws = ctx.queue.create_buffer(&BufferDescriptor {
            size: mesh_state.num_opauqe_instances as u64 * size_of::<DrawIndirectCommand>() as u64,
            usage: BufferUsage::STORAGE | BufferUsage::INDIRECT,
            flags: UsageFlags::empty(),
        });

        let descriptor_set = ctx.queue.create_descriptor_set(&DescriptorSetDescriptor {
            layout: &self.pipeline.builder.descriptor,
            entries: &[
                DescriptorSetEntry {
                    binding: 0,
                    resource: BindingResource::Buffer(instances_in),
                },
                DescriptorSetEntry {
                    binding: 1,
                    resource: BindingResource::Buffer(&instances_out),
                },
                DescriptorSetEntry {
                    binding: 2,
                    resource: BindingResource::Buffer(&draws),
                },
            ],
        });

        let mut compute_pass = ctx.queue.run_compute_pass(&ComputePassDescriptor {
            name: "Drawcall Generation",
        });

        let workgroups = mesh_state.num_opauqe_instances.div_ceil(WORKGROUP_SIZE);

        compute_pass.set_pipeline(&*self.pipeline.get(ctx.queue, TextureFormat::Rgba8Unorm));
        compute_pass.set_push_constants(
            ShaderStages::COMPUTE,
            0,
            bytemuck::bytes_of(&mesh_state.num_opauqe_instances),
        );
        compute_pass.set_descriptor_set(0, &descriptor_set);
        compute_pass.dispatch(workgroups, 1, 1);

        ctx.write(INSTANCE_BUFFER, instances_out).unwrap();
        ctx.write(INDIRECT_DRAW_BUFFER, draws).unwrap();
    }
}

#[derive(Debug)]
struct BuildPipeline {
    descriptor: DescriptorSetLayout,
}

impl PipelineBuilder for BuildPipeline {
    fn build(
        &self,
        queue: &CommandQueue<'_>,
        shaders: &[Shader],
        _format: TextureFormat,
    ) -> Pipeline {
        queue.create_pipeline(&PipelineDescriptor {
            topology: PrimitiveTopology::TriangleList,
            cull_mode: None,
            front_face: FrontFace::Ccw,
            descriptors: &[&self.descriptor],
            stages: &[PipelineStage::Compute(ComputeStage {
                shader: &shaders[0],
                entry: "main",
            })],
            depth_stencil_state: None,
            push_constant_ranges: &[PushConstantRange {
                range: 0..4,
                stages: ShaderStages::COMPUTE,
            }],
        })
    }
}
