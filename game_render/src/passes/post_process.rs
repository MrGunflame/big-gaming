use std::sync::Arc;

use game_common::components::Color;
use game_tracing::trace_span;

use crate::backend::vulkan::{DescriptorSetLayout, Pipeline, Sampler};
use crate::backend::{
    AddressMode, DescriptorBinding, DescriptorSetDescriptor, DescriptorType, FilterMode,
    FragmentStage, FrontFace, LoadOp, PipelineDescriptor, PipelineStage, PrimitiveTopology,
    SamplerDescriptor, ShaderModule, ShaderSource, ShaderStages, StoreOp, TextureFormat,
    VertexStage,
};
use crate::graph::ctx::{
    BindGroupDescriptor, BindGroupEntry, BindingResource, CommandQueue, RenderPassColorAttachment,
    RenderPassDescriptor, Texture,
};
use crate::graph::{Node, RenderContext, SlotLabel};
use crate::pipeline_cache::{PipelineBuilder, PipelineCache};

const SHADER: &str = include_str!("../../shaders/post_process.wgsl");

pub struct PostProcessPass {
    sampler: Arc<Sampler>,
    bind_group_layout: Arc<DescriptorSetLayout>,
    pipelines: PipelineCache<PostProcessPipelineBuilder>,
    src: SlotLabel,
    dst: SlotLabel,
}

impl PostProcessPass {
    pub fn new(queue: &mut CommandQueue<'_>, src: SlotLabel, dst: SlotLabel) -> Self {
        let bind_group_layout = Arc::new(queue.create_descriptor_set_layout(
            &DescriptorSetDescriptor {
                bindings: &[
                    DescriptorBinding {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        kind: DescriptorType::Texture,
                    },
                    DescriptorBinding {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        kind: DescriptorType::Sampler,
                    },
                ],
            },
        ));

        let shader = queue.create_shader_module(ShaderSource::Wgsl(SHADER));

        let sampler = queue.create_sampler(&SamplerDescriptor {
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
        });

        let pipelines = PipelineCache::new(PostProcessPipelineBuilder {
            shader,
            descriptor_set_layout: bind_group_layout.clone(),
        });

        Self {
            bind_group_layout,
            sampler: Arc::new(sampler),
            pipelines,
            src,
            dst,
        }
    }
}

impl Node for PostProcessPass {
    fn render(&self, ctx: &mut RenderContext<'_, '_>) {
        let _span = trace_span!("PostProcessPass::render").entered();

        let input = ctx.read::<Texture>(self.src).unwrap().clone();
        let output = ctx.read::<Texture>(self.dst).unwrap().clone();

        let pipeline = self.pipelines.get(&mut ctx.queue, output.format());

        let bind_group = ctx.queue.create_bind_group(&BindGroupDescriptor {
            layout: &self.bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::Texture(&input),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&self.sampler),
                },
            ],
        });

        let mut render_pass = ctx.queue.run_render_pass(&RenderPassDescriptor {
            color_attachments: &[RenderPassColorAttachment {
                texture: &output,
                load_op: LoadOp::Clear(Color::BLACK),
                store_op: StoreOp::Store,
            }],
        });

        render_pass.set_pipeline(&pipeline);
        render_pass.set_bind_group(0, &bind_group);
        render_pass.draw(0..3, 0..1);
    }
}

#[derive(Debug)]
struct PostProcessPipelineBuilder {
    shader: ShaderModule,
    descriptor_set_layout: Arc<DescriptorSetLayout>,
}

impl PipelineBuilder for PostProcessPipelineBuilder {
    fn build(&self, queue: &mut CommandQueue<'_>, format: TextureFormat) -> Arc<Pipeline> {
        let _span = trace_span!("PostProcessPipelineBuilder::build").entered();

        Arc::new(queue.create_pipeline(&PipelineDescriptor {
            topology: PrimitiveTopology::TriangleList,
            front_face: FrontFace::Ccw,
            cull_mode: None,
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
            push_constant_ranges: &[],
        }))
    }
}
