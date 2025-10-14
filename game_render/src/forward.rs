use std::num::NonZeroU32;
use std::sync::Arc;

use game_common::cell::UnsafeRefCell;

use crate::api::{
    CommandQueue, DescriptorSetLayout, FragmentStage, Pipeline, PipelineDescriptor, PipelineStage,
    Sampler, VertexStage,
};
use crate::backend::{
    AddressMode, ColorTargetState, CompareOp, DepthStencilState, DescriptorBinding,
    DescriptorSetDescriptor, DescriptorType, FilterMode, FrontFace, PrimitiveTopology,
    PushConstantRange, SamplerDescriptor, ShaderStages, TextureFormat,
};
use crate::entities::{Event, Resources};
use crate::pipeline_cache::{PipelineBuilder, PipelineCache};
use crate::shader::{Shader, ShaderConfig, ShaderLanguage, ShaderSource};

const VS_SHADER: &'static str = concat!(env!("CARGO_MANIFEST_DIR"), "/shaders/forward_vs.wgsl");
const FS_SHADER: &'static str = concat!(env!("CARGO_MANIFEST_DIR"), "/shaders/forward_fs.wgsl");

#[derive(Debug)]
pub struct ForwardPipeline {
    pub pipeline: PipelineCache<ForwardPipelineBuilder>,
    pub vs_bind_group_layout: DescriptorSetLayout,
    pub mesh_bind_group_layout: DescriptorSetLayout,
    pub material_bind_group_layout: DescriptorSetLayout,
    pub lights_bind_group_layout: DescriptorSetLayout,
    pub sampler: Sampler,
    pub resources: Arc<Resources>,
    pub events: UnsafeRefCell<Vec<Event>>,
}

impl ForwardPipeline {
    pub fn new(queue: &mut CommandQueue<'_>, resources: Arc<Resources>) -> Self {
        let vs_bind_group_layout = queue.create_descriptor_set_layout(&DescriptorSetDescriptor {
            bindings: &[
                // MODEL
                DescriptorBinding {
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
                    kind: DescriptorType::Uniform,
                    count: NonZeroU32::MIN,
                },
            ],
        });

        let mesh_bind_group_layout = queue.create_descriptor_set_layout(&DescriptorSetDescriptor {
            bindings: &[
                // POSITIONS
                DescriptorBinding {
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
                    kind: DescriptorType::Storage,
                    count: NonZeroU32::MIN,
                },
                // NORMALS
                DescriptorBinding {
                    binding: 1,
                    visibility: ShaderStages::VERTEX,
                    kind: DescriptorType::Storage,
                    count: NonZeroU32::MIN,
                },
                // TANGENTS
                DescriptorBinding {
                    binding: 2,
                    visibility: ShaderStages::VERTEX,
                    kind: DescriptorType::Storage,
                    count: NonZeroU32::MIN,
                },
                // UVS
                DescriptorBinding {
                    binding: 3,
                    visibility: ShaderStages::VERTEX,
                    kind: DescriptorType::Storage,
                    count: NonZeroU32::MIN,
                },
            ],
        });

        let material_bind_group_layout =
            queue.create_descriptor_set_layout(&DescriptorSetDescriptor {
                bindings: &[
                    // MATERIAL CONSTANTS
                    DescriptorBinding {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        kind: DescriptorType::Uniform,
                        count: NonZeroU32::MIN,
                    },
                    // ALBEDO
                    DescriptorBinding {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        kind: DescriptorType::Texture,
                        count: NonZeroU32::MIN,
                    },
                    // NORMAL
                    DescriptorBinding {
                        binding: 2,
                        visibility: ShaderStages::FRAGMENT,
                        kind: DescriptorType::Texture,
                        count: NonZeroU32::MIN,
                    },
                    // METALLIC-ROUGHNESS
                    DescriptorBinding {
                        binding: 3,
                        visibility: ShaderStages::FRAGMENT,
                        kind: DescriptorType::Texture,
                        count: NonZeroU32::MIN,
                    },
                    DescriptorBinding {
                        binding: 4,
                        visibility: ShaderStages::FRAGMENT,
                        kind: DescriptorType::Sampler,
                        count: NonZeroU32::MIN,
                    },
                ],
            });

        let lights_bind_group_layout =
            queue.create_descriptor_set_layout(&&DescriptorSetDescriptor {
                bindings: &[
                    // DIRECTIONAL LIGHTS
                    DescriptorBinding {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        kind: DescriptorType::Storage,
                        count: NonZeroU32::MIN,
                    },
                    // POINT LIGHTS
                    DescriptorBinding {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        kind: DescriptorType::Storage,
                        count: NonZeroU32::MIN,
                    },
                    // SPOT LIGHTS
                    DescriptorBinding {
                        binding: 2,
                        visibility: ShaderStages::FRAGMENT,
                        kind: DescriptorType::Storage,
                        count: NonZeroU32::MIN,
                    },
                ],
            });

        let pipeline = PipelineCache::new(
            ForwardPipelineBuilder {
                vs_bind_group_layout: vs_bind_group_layout.clone(),
                material_bind_group_layout: material_bind_group_layout.clone(),
                mesh_bind_group_layout: mesh_bind_group_layout.clone(),
                lights_bind_group_layout: lights_bind_group_layout.clone(),
            },
            vec![
                ShaderConfig {
                    source: ShaderSource::File(VS_SHADER.into()),
                    language: ShaderLanguage::Wgsl,
                },
                ShaderConfig {
                    source: ShaderSource::File(FS_SHADER.into()),
                    language: ShaderLanguage::Wgsl,
                },
            ],
        );

        let sampler = queue.create_sampler(&SamplerDescriptor {
            address_mode_u: AddressMode::Repeat,
            address_mode_v: AddressMode::Repeat,
            address_mode_w: AddressMode::Repeat,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
        });

        Self {
            pipeline,
            vs_bind_group_layout: vs_bind_group_layout,
            mesh_bind_group_layout: mesh_bind_group_layout,
            material_bind_group_layout: material_bind_group_layout,
            lights_bind_group_layout: lights_bind_group_layout,
            sampler: sampler,
            resources,
            events: UnsafeRefCell::new(Vec::new()),
        }
    }
}

#[derive(Debug)]
pub struct ForwardPipelineBuilder {
    vs_bind_group_layout: DescriptorSetLayout,
    mesh_bind_group_layout: DescriptorSetLayout,
    material_bind_group_layout: DescriptorSetLayout,
    lights_bind_group_layout: DescriptorSetLayout,
}

impl PipelineBuilder for ForwardPipelineBuilder {
    fn build(
        &self,
        queue: &CommandQueue<'_>,
        shaders: &[Shader],
        format: TextureFormat,
    ) -> Pipeline {
        queue.create_pipeline(&PipelineDescriptor {
            topology: PrimitiveTopology::TriangleList,
            cull_mode: None,
            front_face: FrontFace::Ccw,
            descriptors: &[
                &self.vs_bind_group_layout,
                &self.mesh_bind_group_layout,
                &self.material_bind_group_layout,
                &self.lights_bind_group_layout,
            ],
            stages: &[
                PipelineStage::Vertex(VertexStage {
                    shader: &shaders[0],
                    entry: "vs_main",
                }),
                PipelineStage::Fragment(FragmentStage {
                    shader: &shaders[1],
                    entry: "fs_main",
                    targets: &[ColorTargetState {
                        format,
                        blend: None,
                    }],
                }),
            ],
            depth_stencil_state: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare_op: CompareOp::Less,
            }),
            push_constant_ranges: &[PushConstantRange {
                range: 0..128,
                stages: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
            }],
        })
    }
}
