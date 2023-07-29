use bevy_ecs::system::Resource;
use bevy_ecs::world::FromWorld;
use wgpu::{
    BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, BlendState,
    BufferBindingType, ColorTargetState, ColorWrites, Device, Face, FragmentState, FrontFace,
    MultisampleState, PipelineLayoutDescriptor, PolygonMode, PrimitiveState, PrimitiveTopology,
    RenderPipeline, RenderPipelineDescriptor, ShaderModuleDescriptor, ShaderSource, ShaderStages,
    TextureFormat, VertexState,
};

use crate::mesh::Vertex;
use crate::RenderDevice;

#[derive(Debug, Resource)]
pub struct ForwardPipeline {
    pub pipeline: RenderPipeline,
    pub vs_bind_group_layout: BindGroupLayout,
    pub fs_bind_group_layout: BindGroupLayout,
    pub mesh_bind_group_layout: BindGroupLayout,
}

impl FromWorld for ForwardPipeline {
    fn from_world(world: &mut bevy_ecs::world::World) -> Self {
        world.resource_scope::<RenderDevice, _>(|_, device| Self::new(&device))
    }
}

impl ForwardPipeline {
    pub fn new(device: &Device) -> Self {
        let vs_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("vs_bind_group_layout"),
            entries: &[
                // CAMERA
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
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
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
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

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("foward_pipeline_layout"),
            bind_group_layouts: &[&vs_bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("forward_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &vs_shader,
                entry_point: "vs_main",
                buffers: &[Vertex::layout()],
            },
            fragment: Some(FragmentState {
                module: &fs_shader,
                entry_point: "fs_main",
                targets: &[Some(ColorTargetState {
                    format: TextureFormat::Bgra8Unorm,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                polygon_mode: PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        Self {
            pipeline,
            vs_bind_group_layout,
            fs_bind_group_layout,
            mesh_bind_group_layout,
        }
    }
}
