use std::sync::Arc;

use game_common::cell::UnsafeRefCell;

use crate::backend::vulkan::{DescriptorSetLayout, Pipeline, Sampler};
use crate::backend::{
    AddressMode, CompareOp, DepthStencilState, DescriptorBinding, DescriptorSetDescriptor,
    DescriptorType, Face, FilterMode, FragmentStage, FrontFace, PipelineDescriptor, PipelineStage,
    PrimitiveTopology, PushConstantRange, SamplerDescriptor, ShaderModuleDescriptor, ShaderSource,
    ShaderStages, TextureFormat, VertexStage,
};
use crate::entities::{Event, Resources};
use crate::graph::ctx::CommandQueue;

#[derive(Debug)]
pub struct ForwardPipeline {
    pub pipeline: Arc<Pipeline>,
    pub vs_bind_group_layout: Arc<DescriptorSetLayout>,
    pub mesh_bind_group_layout: Arc<DescriptorSetLayout>,
    pub material_bind_group_layout: Arc<DescriptorSetLayout>,
    pub lights_bind_group_layout: Arc<DescriptorSetLayout>,
    pub sampler: Arc<Sampler>,
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
                },
                // NORMALS
                DescriptorBinding {
                    binding: 1,
                    visibility: ShaderStages::VERTEX,
                    kind: DescriptorType::Storage,
                },
                // TANGENTS
                DescriptorBinding {
                    binding: 2,
                    visibility: ShaderStages::VERTEX,
                    kind: DescriptorType::Storage,
                },
                // UVS
                DescriptorBinding {
                    binding: 3,
                    visibility: ShaderStages::VERTEX,
                    kind: DescriptorType::Storage,
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
                    },
                    // ALBEDO
                    DescriptorBinding {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        kind: DescriptorType::Texture,
                    },
                    // NORMAL
                    DescriptorBinding {
                        binding: 2,
                        visibility: ShaderStages::FRAGMENT,
                        kind: DescriptorType::Texture,
                    },
                    // METALLIC-ROUGHNESS
                    DescriptorBinding {
                        binding: 3,
                        visibility: ShaderStages::FRAGMENT,
                        kind: DescriptorType::Texture,
                    },
                    DescriptorBinding {
                        binding: 4,
                        visibility: ShaderStages::FRAGMENT,
                        kind: DescriptorType::Sampler,
                    },
                ],
            });

        let vs_shader = queue.create_shader_module(ShaderSource::Wgsl(include_str!(
            "../shaders/forward_vs.wgsl"
        )));

        let fs_shader = queue.create_shader_module(ShaderSource::Wgsl(include_str!(
            "../shaders/forward_fs.wgsl"
        )));

        let lights_bind_group_layout =
            queue.create_descriptor_set_layout(&&DescriptorSetDescriptor {
                bindings: &[
                    // DIRECTIONAL LIGHTS
                    DescriptorBinding {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        kind: DescriptorType::Storage,
                    },
                    // POINT LIGHTS
                    DescriptorBinding {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        kind: DescriptorType::Storage,
                    },
                    // SPOT LIGHTS
                    DescriptorBinding {
                        binding: 2,
                        visibility: ShaderStages::FRAGMENT,
                        kind: DescriptorType::Storage,
                    },
                ],
            });

        // let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        //     label: Some("foward_pipeline_layout"),
        //     bind_group_layouts: &[
        //         &vs_bind_group_layout,
        //         &mesh_bind_group_layout,
        //         &material_bind_group_layout,
        //         &lights_bind_group_layout,
        //     ],
        //     push_constant_ranges: &[PushConstantRange {
        //         stages: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
        //         range: 0..128,
        //     }],
        // });

        let pipeline = queue.create_pipeline(&PipelineDescriptor {
            topology: PrimitiveTopology::TriangleList,
            cull_mode: None,
            front_face: FrontFace::Ccw,
            descriptors: &[
                &vs_bind_group_layout,
                &mesh_bind_group_layout,
                &material_bind_group_layout,
                &lights_bind_group_layout,
            ],
            stages: &[
                PipelineStage::Vertex(VertexStage {
                    shader: &vs_shader,
                    entry: "vs_main",
                }),
                PipelineStage::Fragment(FragmentStage {
                    shader: &fs_shader,
                    entry: "fs_main",
                    targets: &[TextureFormat::Rgba16Float],
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
        });

        // let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
        //     label: Some("forward_pipeline"),
        //     layout: Some(&pipeline_layout),
        //     vertex: VertexState {
        //         module: &vs_shader,
        //         entry_point: "vs_main",
        //         buffers: &[],
        //     },
        //     fragment: Some(FragmentState {
        //         module: &fs_shader,
        //         entry_point: "fs_main",
        //         targets: &[Some(ColorTargetState {
        //             format: TextureFormat::Rgba16Float,
        //             blend: Some(BlendState::ALPHA_BLENDING),
        //             write_mask: ColorWrites::ALL,
        //         })],
        //     }),
        //     primitive: PrimitiveState {
        //         topology: PrimitiveTopology::TriangleList,
        //         strip_index_format: None,
        //         front_face: FrontFace::Ccw,
        //         cull_mode: None,
        //         polygon_mode: PolygonMode::Fill,
        //         unclipped_depth: false,
        //         conservative: false,
        //     },
        //     depth_stencil: Some(DepthStencilState {
        //         format: DEPTH_TEXTURE_FORMAT,
        //         depth_write_enabled: true,
        //         depth_compare: CompareFunction::Less,
        //         stencil: StencilState::default(),
        //         bias: DepthBiasState::default(),
        //     }),
        //     multisample: MultisampleState {
        //         count: 1,
        //         mask: !0,
        //         alpha_to_coverage_enabled: false,
        //     },
        //     multiview: None,
        // });

        let sampler = queue.create_sampler(&SamplerDescriptor {
            address_mode_u: AddressMode::Repeat,
            address_mode_v: AddressMode::Repeat,
            address_mode_w: AddressMode::Repeat,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            // mipmap_filter: FilterMode::Linear,
            // lod_min_clamp: 0.0,
            // lod_max_clamp: 100.0,
            // ..Default::default()
        });

        Self {
            pipeline: Arc::new(pipeline),
            vs_bind_group_layout: Arc::new(vs_bind_group_layout),
            mesh_bind_group_layout: Arc::new(mesh_bind_group_layout),
            material_bind_group_layout: Arc::new(material_bind_group_layout),
            lights_bind_group_layout: Arc::new(lights_bind_group_layout),
            sampler: Arc::new(sampler),
            resources,
            events: UnsafeRefCell::new(Vec::new()),
        }
    }
}
