use wgpu::{
    AddressMode, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType,
    BlendState, BufferBindingType, ColorTargetState, ColorWrites, CompareFunction, DepthBiasState,
    DepthStencilState, Device, Face, FilterMode, FragmentState, FrontFace, MultisampleState,
    PipelineLayoutDescriptor, PolygonMode, PrimitiveState, PrimitiveTopology, RenderPipeline,
    RenderPipelineDescriptor, Sampler, SamplerBindingType, SamplerDescriptor,
    ShaderModuleDescriptor, ShaderSource, ShaderStages, StencilState, TextureFormat,
    TextureSampleType, TextureViewDimension, VertexState,
};

use crate::depth_stencil::DEPTH_TEXTURE_FORMAT;
use crate::pbr::material::DefaultTextures;
use crate::texture::Images;

#[derive(Debug)]
pub struct ForwardPipeline {
    pub pipeline: RenderPipeline,
    pub vs_bind_group_layout: BindGroupLayout,
    pub fs_bind_group_layout: BindGroupLayout,
    pub mesh_bind_group_layout: BindGroupLayout,
    pub material_bind_group_layout: BindGroupLayout,
    pub lights_bind_group_layout: BindGroupLayout,
    pub sampler: Sampler,
    pub default_textures: DefaultTextures,
}

impl ForwardPipeline {
    pub fn new(device: &Device, images: &mut Images) -> Self {
        let vs_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("vs_bind_group_layout"),
            entries: &[
                // CAMERA
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // MODEL
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let mesh_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("mesh_bind_group_layout"),
            entries: &[
                // POSITIONS
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // NORMALS
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
                // TANGENTS
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // UVS
                BindGroupLayoutEntry {
                    binding: 3,
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

        let material_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("material_bind_group_layout"),
                entries: &[
                    // MATERIAL CONSTANTS
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // ALBEDO
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // NORMAL
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // METALLIC-ROUGHNESS
                    BindGroupLayoutEntry {
                        binding: 3,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 4,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let vs_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("forward_vs"),
            source: ShaderSource::Wgsl(include_str!("../shaders/forward_vs.wgsl").into()),
        });

        let fs_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("forward_fs"),
            source: ShaderSource::Wgsl(include_str!("../shaders/forward_fs.wgsl").into()),
        });

        let fs_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("fs_bind_group_layout"),
            entries: &[],
        });

        let lights_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("lights_bind_group_layout"),
                entries: &[
                    // DIRECTIONAL LIGHTS
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // POINT LIGHTS
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // SPOT LIGHTS
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::FRAGMENT,
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
            label: Some("foward_pipeline_layout"),
            bind_group_layouts: &[
                &vs_bind_group_layout,
                &mesh_bind_group_layout,
                &material_bind_group_layout,
                &lights_bind_group_layout,
            ],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("forward_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &vs_shader,
                entry_point: "vs_main",
                buffers: &[],
            },
            fragment: Some(FragmentState {
                module: &fs_shader,
                entry_point: "fs_main",
                targets: &[Some(ColorTargetState {
                    format: TextureFormat::Rgba16Float,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(DepthStencilState {
                format: DEPTH_TEXTURE_FORMAT,
                depth_write_enabled: true,
                depth_compare: CompareFunction::Less,
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            }),
            multisample: MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("default_sampler"),
            address_mode_u: AddressMode::Repeat,
            address_mode_v: AddressMode::Repeat,
            address_mode_w: AddressMode::Repeat,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            lod_min_clamp: 0.0,
            lod_max_clamp: 100.0,
            ..Default::default()
        });

        Self {
            pipeline,
            vs_bind_group_layout,
            fs_bind_group_layout,
            mesh_bind_group_layout,
            material_bind_group_layout,
            lights_bind_group_layout,
            sampler,
            default_textures: DefaultTextures::new(images),
        }
    }
}
