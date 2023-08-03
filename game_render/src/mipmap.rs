use std::collections::HashMap;

use wgpu::{
    AddressMode, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, Color, ColorTargetState, ColorWrites,
    CommandEncoder, Device, FilterMode, FragmentState, FrontFace, LoadOp, MultisampleState,
    Operations, PipelineLayout, PipelineLayoutDescriptor, PolygonMode, PrimitiveState,
    PrimitiveTopology, RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline,
    RenderPipelineDescriptor, Sampler, SamplerBindingType, SamplerDescriptor, ShaderModule,
    ShaderModuleDescriptor, ShaderSource, ShaderStages, Texture, TextureDescriptor, TextureFormat,
    TextureSampleType, TextureViewDescriptor, TextureViewDimension, VertexState,
};

#[derive(Debug)]
pub struct MipMapGenerator {
    // FIXME: Maybe split texture bind group from sampler.
    bind_group_layout: BindGroupLayout,
    sampler: Sampler,
    pipeline_layout: PipelineLayout,
    pipelines: HashMap<TextureFormat, RenderPipeline>,
    shader: ShaderModule,
}

impl MipMapGenerator {
    pub fn new(device: &Device) -> Self {
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("mipmap_bind_group_layout"),
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
            label: Some("mipmap_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("mipmap_shader"),
            source: ShaderSource::Wgsl(include_str!("../shaders/mipmap.wgsl").into()),
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("mipmap_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(ColorTargetState {
                    format: TextureFormat::Rgba8UnormSrgb,
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
        });

        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("mipmap_sampler"),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            lod_min_clamp: 0.0,
            lod_max_clamp: 100.0,
            compare: None,
            anisotropy_clamp: 1,
            border_color: None,
        });

        Self {
            bind_group_layout,
            sampler,
            pipeline_layout,
            shader,
            pipelines: HashMap::new(),
        }
    }

    pub fn generate_mipmaps(
        &mut self,
        device: &Device,
        encoder: &mut CommandEncoder,
        texture: &Texture,
        desc: &TextureDescriptor,
    ) {
        let pipeline = match self.pipelines.get(&texture.format()) {
            Some(pl) => pl,
            None => {
                self.build_pipeline(device, texture.format());
                self.pipelines.get(&texture.format()).unwrap()
            }
        };

        let mut mips = Vec::new();
        for mip_level in 0..desc.size.max_mips(desc.dimension) {
            let mip = texture.create_view(&TextureViewDescriptor {
                label: None,
                base_mip_level: mip_level,
                mip_level_count: Some(1),
                ..Default::default()
            });

            mips.push(mip);
        }

        for (index, views) in mips.windows(2).enumerate() {
            let src_view = &views[0];
            let dst_view = &views[1];

            let bind_group = device.create_bind_group(&BindGroupDescriptor {
                label: Some("mipmap_bind_group"),
                layout: &self.bind_group_layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::TextureView(src_view),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::Sampler(&self.sampler),
                    },
                ],
            });

            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: dst_view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::BLACK),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            render_pass.set_pipeline(&pipeline);
            render_pass.set_bind_group(0, &bind_group, &[]);
            render_pass.draw(0..3, 0..1);
        }
    }

    fn build_pipeline(&mut self, device: &Device, format: TextureFormat) {
        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
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
        });

        self.pipelines.insert(format, pipeline);
    }
}
