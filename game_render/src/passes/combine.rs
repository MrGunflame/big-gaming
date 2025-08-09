use std::num::NonZeroU32;

use game_tracing::trace_span;
use glam::UVec2;

use crate::api::{
    BindingResource, CommandQueue, ComputePassDescriptor, DescriptorSetDescriptor,
    DescriptorSetEntry, DescriptorSetLayout, DescriptorSetLayoutDescriptor, Pipeline,
    PipelineDescriptor, Sampler, Texture, TextureViewDescriptor,
};
use crate::backend::{
    AddressMode, ComputeStage, DescriptorBinding, DescriptorType, FilterMode, FrontFace,
    PipelineStage, PrimitiveTopology, PushConstantRange, SamplerDescriptor, ShaderStages,
    TextureFormat,
};
use crate::graph::{Node, RenderContext, SlotLabel};
use crate::pipeline_cache::{PipelineBuilder, PipelineCache};
use crate::shader::{Shader, ShaderConfig, ShaderLanguage, ShaderSource};

const SHADER: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/shaders/combine.slang");

const TILE_SIZE: UVec2 = UVec2::new(16, 16);

pub struct CombinePass {
    inputs: Vec<SlotLabel>,
    output: SlotLabel,
    sampler: Sampler,
    pipeline: PipelineCache<CombinePipelineBuilder>,
}

impl CombinePass {
    pub fn new(queue: &CommandQueue<'_>, inputs: Vec<SlotLabel>, output: SlotLabel) -> Self {
        let sampler = queue.create_sampler(&SamplerDescriptor {
            address_mode_u: AddressMode::Repeat,
            address_mode_v: AddressMode::Repeat,
            address_mode_w: AddressMode::Repeat,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
        });

        let descriptor = queue.create_descriptor_set_layout(&DescriptorSetLayoutDescriptor {
            bindings: &[
                DescriptorBinding {
                    binding: 0,
                    visibility: ShaderStages::COMPUTE,
                    kind: DescriptorType::Texture,
                    count: NonZeroU32::new(inputs.len() as u32).unwrap(),
                },
                DescriptorBinding {
                    binding: 1,
                    visibility: ShaderStages::COMPUTE,
                    kind: DescriptorType::Texture,
                    count: NonZeroU32::MIN,
                },
                DescriptorBinding {
                    binding: 2,
                    visibility: ShaderStages::COMPUTE,
                    kind: DescriptorType::Sampler,
                    count: NonZeroU32::MIN,
                },
            ],
        });

        Self {
            inputs,
            output,
            sampler,
            pipeline: PipelineCache::new(
                CombinePipelineBuilder { descriptor },
                vec![ShaderConfig {
                    source: ShaderSource::File(SHADER.into()),
                    language: ShaderLanguage::Slang,
                }],
            ),
        }
    }
}

impl Node for CombinePass {
    fn render<'a>(&self, ctx: &'a mut RenderContext<'_, 'a>) {
        let _span = trace_span!("CombinePass").entered();

        let mut inputs = Vec::with_capacity(self.inputs.len());
        for input in &self.inputs {
            let input = ctx.read::<Texture>(*input).unwrap();
            inputs.push(input.create_view(&TextureViewDescriptor::default()));
        }

        let inputs = inputs.iter().collect::<Vec<_>>();

        let output = ctx.read::<Texture>(self.output).unwrap();
        let output_view = output.create_view(&TextureViewDescriptor::default());

        let descriptor = ctx.queue.create_descriptor_set(&DescriptorSetDescriptor {
            layout: &self.pipeline.builder.descriptor,
            entries: &[
                DescriptorSetEntry {
                    binding: 0,
                    resource: BindingResource::TextureArray(&inputs),
                },
                DescriptorSetEntry {
                    binding: 1,
                    resource: BindingResource::Texture(&output_view),
                },
                DescriptorSetEntry {
                    binding: 2,
                    resource: BindingResource::Sampler(&self.sampler),
                },
            ],
        });

        let pipeline = self.pipeline.get(ctx.queue, TextureFormat::Rgba8Unorm);

        let mut pass = ctx.queue.run_compute_pass(&ComputePassDescriptor {
            name: "Combine Pass",
        });

        pass.set_pipeline(&pipeline);
        pass.set_push_constants(ShaderStages::COMPUTE, 0, bytemuck::bytes_of(&output.size()));
        pass.set_descriptor_set(0, &descriptor);
        pass.dispatch(
            output.size().x.div_ceil(TILE_SIZE.x),
            output.size().y.div_ceil(TILE_SIZE.y),
            1,
        );
    }
}

struct CombinePipelineBuilder {
    descriptor: DescriptorSetLayout,
}

impl PipelineBuilder for CombinePipelineBuilder {
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
                range: 0..8,
                stages: ShaderStages::COMPUTE,
            }],
        })
    }
}
