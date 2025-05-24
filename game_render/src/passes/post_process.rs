use std::num::NonZeroU32;

use game_common::components::Color;
use game_tracing::trace_span;

use crate::api::{
    BindingResource, CommandQueue, DescriptorSetEntry, DescriptorSetLayout, Pipeline,
    PipelineDescriptor, RenderPassColorAttachment, RenderPassDescriptor, Sampler, Texture,
    TextureViewDescriptor,
};
use crate::backend::{
    AddressMode, ColorTargetState, DescriptorBinding, DescriptorSetDescriptor, DescriptorType,
    FilterMode, FragmentStage, FrontFace, LoadOp, PipelineStage, PrimitiveTopology,
    SamplerDescriptor, ShaderStages, StoreOp, TextureFormat, VertexStage,
};
use crate::graph::{Node, RenderContext, SlotLabel};
use crate::pipeline_cache::{PipelineBuilder, PipelineCache};
use crate::shader::{Shader, ShaderConfig, ShaderLanguage, ShaderSource};

const SHADER: &'static str = concat!(env!("CARGO_MANIFEST_DIR"), "/shaders/post_process.wgsl");

pub struct PostProcessPass {
    sampler: Sampler,
    bind_group_layout: DescriptorSetLayout,
    pipelines: PipelineCache<PostProcessPipelineBuilder>,
    src: SlotLabel,
    dst: SlotLabel,
}

impl PostProcessPass {
    pub fn new(queue: &mut CommandQueue<'_>, src: SlotLabel, dst: SlotLabel) -> Self {
        let bind_group_layout = queue.create_descriptor_set_layout(&DescriptorSetDescriptor {
            bindings: &[
                DescriptorBinding {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    kind: DescriptorType::Texture,
                    count: NonZeroU32::MIN,
                },
                DescriptorBinding {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    kind: DescriptorType::Sampler,
                    count: NonZeroU32::MIN,
                },
            ],
        });

        let sampler = queue.create_sampler(&SamplerDescriptor {
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
        });

        let pipelines = PipelineCache::new(
            PostProcessPipelineBuilder {
                descriptor_set_layout: bind_group_layout.clone(),
            },
            vec![ShaderConfig {
                source: ShaderSource::File(SHADER.into()),
                language: ShaderLanguage::Wgsl,
            }],
        );

        Self {
            bind_group_layout,
            sampler,
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

        let bind_group = ctx
            .queue
            .create_descriptor_set(&crate::api::DescriptorSetDescriptor {
                layout: &self.bind_group_layout,
                entries: &[
                    DescriptorSetEntry {
                        binding: 0,
                        resource: BindingResource::Texture(
                            &input.create_view(&TextureViewDescriptor::default()),
                        ),
                    },
                    DescriptorSetEntry {
                        binding: 1,
                        resource: BindingResource::Sampler(&self.sampler),
                    },
                ],
            });

        let mut render_pass = ctx.queue.run_render_pass(&RenderPassDescriptor {
            color_attachments: &[RenderPassColorAttachment {
                target: &output.create_view(&TextureViewDescriptor::default()),
                load_op: LoadOp::Clear(Color::BLACK),
                store_op: StoreOp::Store,
            }],
            depth_stencil_attachment: None,
        });

        render_pass.set_pipeline(&pipeline);
        render_pass.set_descriptor_set(0, &bind_group);
        render_pass.draw(0..3, 0..1);
    }
}

#[derive(Debug)]
struct PostProcessPipelineBuilder {
    descriptor_set_layout: DescriptorSetLayout,
}

impl PipelineBuilder for PostProcessPipelineBuilder {
    fn build(
        &self,
        queue: &CommandQueue<'_>,
        shaders: &[Shader],
        format: TextureFormat,
    ) -> Pipeline {
        let _span = trace_span!("PostProcessPipelineBuilder::build").entered();

        queue.create_pipeline(&PipelineDescriptor {
            topology: PrimitiveTopology::TriangleList,
            front_face: FrontFace::Ccw,
            cull_mode: None,
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
