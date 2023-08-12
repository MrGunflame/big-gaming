use std::collections::HashMap;

use bevy_ecs::system::Resource;
use bevy_ecs::world::{FromWorld, World};
use game_render::RenderDevice;
use parking_lot::Mutex;
use wgpu::{
    AddressMode, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType,
    BlendState, ColorTargetState, ColorWrites, Device, Face, FilterMode, FragmentState, FrontFace,
    MultisampleState, PipelineLayout, PipelineLayoutDescriptor, PolygonMode, PrimitiveState,
    PrimitiveTopology, RenderPipeline, RenderPipelineDescriptor, Sampler, SamplerBindingType,
    SamplerDescriptor, ShaderModule, ShaderModuleDescriptor, ShaderSource, ShaderStages,
    TextureFormat, TextureSampleType, TextureViewDimension, VertexState,
};

use super::Vertex;

#[derive(Debug, Resource)]
pub(crate) struct UiPipeline {
    pub bind_group_layout: BindGroupLayout,
    pub sampler: Sampler,
    pipeline_layout: PipelineLayout,
    shader: ShaderModule,
    pipelines: Mutex<HashMap<TextureFormat, RenderPipeline>>,
}

impl UiPipeline {
    pub(crate) fn new(device: &Device) -> Self {
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("ui_bind_group_layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("ui_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("ui_shader"),
            source: ShaderSource::Wgsl(include_str!("../../shaders/ui.wgsl").into()),
        });

        let sampler = device.create_sampler(&SamplerDescriptor {
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
            mipmap_filter: FilterMode::Nearest,
            ..Default::default()
        });

        Self {
            pipeline_layout,
            shader,
            bind_group_layout,
            sampler,
            pipelines: Mutex::new(HashMap::new()),
        }
    }

    pub(crate) fn get(
        &self,
        device: &Device,
        format: TextureFormat,
    ) -> &Mutex<HashMap<TextureFormat, RenderPipeline>> {
        let mut pls = self.pipelines.lock();
        if !pls.contains_key(&format) {
            self.build_pipeline(&mut pls, device, format);
        }

        &self.pipelines
    }

    fn build_pipeline(
        &self,
        pipelines: &mut HashMap<TextureFormat, RenderPipeline>,
        device: &Device,
        format: TextureFormat,
    ) {
        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: None,
            layout: Some(&self.pipeline_layout),
            vertex: VertexState {
                module: &self.shader,
                entry_point: "vs_main",
                buffers: &[Vertex::layout()],
            },
            fragment: Some(FragmentState {
                module: &self.shader,
                entry_point: "fs_main",
                targets: &[Some(ColorTargetState {
                    format,
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
            multisample: MultisampleState::default(),
            multiview: None,
        });

        pipelines.insert(format, pipeline);
    }
}

impl FromWorld for UiPipeline {
    fn from_world(world: &mut World) -> Self {
        world.resource_scope::<RenderDevice, Self>(|_, device| Self::new(&device.0))
    }
}
