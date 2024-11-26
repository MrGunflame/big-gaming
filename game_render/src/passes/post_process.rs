use game_tracing::trace_span;
use wgpu::{
    AddressMode, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, Color, ColorTargetState, ColorWrites,
    Device, FilterMode, FragmentState, FrontFace, LoadOp, MultisampleState, Operations,
    PipelineLayout, PipelineLayoutDescriptor, PolygonMode, PrimitiveState, PrimitiveTopology,
    RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor,
    Sampler, SamplerBindingType, SamplerDescriptor, ShaderModule, ShaderModuleDescriptor,
    ShaderSource, ShaderStages, StoreOp, Texture, TextureFormat, TextureSampleType,
    TextureViewDescriptor, TextureViewDimension, VertexState,
};

use crate::graph::{Node, RenderContext, SlotLabel};
use crate::pipeline_cache::{PipelineBuilder, PipelineCache};

const SHADER: &str = include_str!("../../shaders/post_process.wgsl");

pub struct PostProcessPass {
    sampler: Sampler,
    bind_group_layout: BindGroupLayout,
    pipelines: PipelineCache<PostProcessPipelineBuilder>,
    src: SlotLabel,
    dst: SlotLabel,
}

impl PostProcessPass {
    pub fn new(device: &Device, src: SlotLabel, dst: SlotLabel) -> Self {
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("post_process_bind_group_layout"),
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
            label: Some("post_process_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("post_process_shader"),
            source: ShaderSource::Wgsl(SHADER.into()),
        });

        let sampler = device.create_sampler(&SamplerDescriptor {
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Nearest,
            ..Default::default()
        });

        let pipelines = PipelineCache::new(PostProcessPipelineBuilder {
            pipeline_layout,
            shader,
        });

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

        let input = ctx.read::<Texture>(self.src).unwrap();
        let output = ctx.read::<Texture>(self.dst).unwrap();

        let src = input.create_view(&TextureViewDescriptor::default());
        let dst = output.create_view(&TextureViewDescriptor::default());

        let pipeline = self.pipelines.get(ctx.device, output.format());

        let bind_group = ctx.device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &self.bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&src),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&self.sampler),
                },
            ],
        });

        let mut render_pass = ctx.encoder.begin_render_pass(&RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &dst,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color::BLACK),
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        render_pass.set_pipeline(&pipeline);
        render_pass.set_bind_group(0, &bind_group, &[]);
        render_pass.draw(0..3, 0..1);
    }
}

#[derive(Debug)]
struct PostProcessPipelineBuilder {
    pipeline_layout: PipelineLayout,
    shader: ShaderModule,
}

impl PipelineBuilder for PostProcessPipelineBuilder {
    fn build(&self, device: &Device, format: TextureFormat) -> RenderPipeline {
        let _span = trace_span!("PostProcessPipelineBuilder::build").entered();

        device.create_render_pipeline(&RenderPipelineDescriptor {
            label: None,
            layout: Some(&self.pipeline_layout),
            vertex: VertexState {
                module: &self.shader,
                entry_point: "vs_main",
                buffers: &[],
            },
            fragment: Some(FragmentState {
                module: &self.shader,
                entry_point: "fs_main",
                targets: &[Some(ColorTargetState {
                    format,
                    blend: None,
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
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview: None,
        })
    }
}
