use std::num::NonZeroU32;

use game_common::components::Color;
use game_tracing::trace_span;

use crate::api::{
    BindingResource, CommandQueue, DescriptorSetDescriptor, DescriptorSetEntry,
    DescriptorSetLayout, DescriptorSetLayoutDescriptor, Pipeline, PipelineDescriptor,
    RenderPassColorAttachment, RenderPassDescriptor, Sampler, Texture, TextureViewDescriptor,
};
use crate::backend::{
    AddressMode, DescriptorBinding, DescriptorType, Face, FilterMode, FragmentStage, FrontFace,
    LoadOp, PipelineStage, PrimitiveTopology, SamplerDescriptor, ShaderModule, ShaderStages,
    StoreOp, TextureFormat, VertexStage,
};
use crate::pipeline_cache::{PipelineBuilder, PipelineCache};
use crate::shader::{ShaderConfig, ShaderLanguage, ShaderSource};

const SHADER: &'static str = concat!(env!("CARGO_MANIFEST_DIR"), "/shaders/mipmap.wgsl");

#[derive(Debug)]
pub struct MipMapGenerator {
    layout: DescriptorSetLayout,
    sampler: Sampler,
    pipelines: PipelineCache<BlitPipelineBuilder>,
}

impl MipMapGenerator {
    pub fn new(queue: &mut CommandQueue<'_>) -> Self {
        let layout = queue.create_descriptor_set_layout(&DescriptorSetLayoutDescriptor {
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
            min_filter: FilterMode::Linear,
            mag_filter: FilterMode::Linear,
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mipmap_filter: FilterMode::Linear,
        });

        Self {
            layout: layout.clone(),
            sampler,
            pipelines: PipelineCache::new(
                BlitPipelineBuilder { layout },
                vec![ShaderConfig {
                    source: ShaderSource::File(SHADER.into()),
                    language: ShaderLanguage::Wgsl,
                }],
            ),
        }
    }

    pub fn generate_mipmaps(&self, queue: &mut CommandQueue<'_>, texture: &Texture) {
        let _span = trace_span!("MipMapGenerator::generate_mipmaps").entered();

        let pipeline = self.pipelines.get(queue, texture.format());

        let mut mips = Vec::new();
        for mip_level in 0..texture.mip_levels() {
            let mip = texture.create_view(&TextureViewDescriptor {
                base_mip_level: mip_level,
                mip_levels: Some(1),
            });

            mips.push(mip);
        }

        for views in mips.windows(2) {
            let src_view = &views[0];
            let dst_view = &views[1];

            let set = queue.create_descriptor_set(&DescriptorSetDescriptor {
                layout: &self.layout,
                entries: &[
                    DescriptorSetEntry {
                        binding: 0,
                        resource: BindingResource::Texture(src_view),
                    },
                    DescriptorSetEntry {
                        binding: 1,
                        resource: BindingResource::Sampler(&self.sampler),
                    },
                ],
            });

            let mut render_pass = queue.run_render_pass(&RenderPassDescriptor {
                color_attachments: &[RenderPassColorAttachment {
                    target: dst_view,
                    load_op: LoadOp::Clear(Color::BLACK),
                    store_op: StoreOp::Store,
                }],
                depth_stencil_attachment: None,
            });

            render_pass.set_pipeline(&pipeline);
            render_pass.set_descriptor_set(0, &set);
            render_pass.draw(0..3, 0..1);
        }
    }
}

#[derive(Debug)]
struct BlitPipelineBuilder {
    layout: DescriptorSetLayout,
}

impl PipelineBuilder for BlitPipelineBuilder {
    fn build(
        &self,
        queue: &mut CommandQueue<'_>,
        shaders: &[ShaderModule],
        format: TextureFormat,
    ) -> Pipeline {
        queue.create_pipeline(&PipelineDescriptor {
            topology: PrimitiveTopology::TriangleList,
            front_face: FrontFace::Ccw,
            cull_mode: Some(Face::Back),
            descriptors: &[&self.layout],
            depth_stencil_state: None,
            push_constant_ranges: &[],
            stages: &[
                PipelineStage::Vertex(VertexStage {
                    shader: &shaders[0],
                    entry: "vs_main",
                }),
                PipelineStage::Fragment(FragmentStage {
                    shader: &shaders[0],
                    entry: "fs_main",
                    targets: &[format],
                }),
            ],
        })
    }
}
