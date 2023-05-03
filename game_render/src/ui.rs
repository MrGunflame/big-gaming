use bytemuck::{Pod, Zeroable};
use glam::Vec2;
use image::{ImageBuffer, Rgba};
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{
    AddressMode, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType, BlendState,
    Buffer, BufferAddress, BufferUsages, ColorTargetState, ColorWrites, CommandEncoder, Device,
    Extent3d, Face, FilterMode, FragmentState, FrontFace, ImageCopyTexture, ImageDataLayout,
    IndexFormat, LoadOp, MultisampleState, Operations, Origin3d, PipelineLayoutDescriptor,
    PolygonMode, PrimitiveState, PrimitiveTopology, Queue, RenderPassColorAttachment,
    RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor, Sampler, SamplerBindingType,
    SamplerDescriptor, ShaderModuleDescriptor, ShaderSource, ShaderStages, Texture, TextureAspect,
    TextureDescriptor, TextureDimension, TextureFormat, TextureSampleType, TextureUsages,
    TextureView, TextureViewDescriptor, TextureViewDimension, VertexAttribute, VertexBufferLayout,
    VertexFormat, VertexState, VertexStepMode,
};

use crate::layout::{Frame, Layout};

#[derive(Debug)]
pub struct UiPipeline {
    bind_group_layout: BindGroupLayout,
    pipeline: RenderPipeline,
    sampler: Sampler,
}

impl UiPipeline {
    pub fn new(device: &Device) -> Self {
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("ui_layout"),
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
            source: ShaderSource::Wgsl(include_str!("ui.wgsl").into()),
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("ui_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(ColorTargetState {
                    format: TextureFormat::Bgra8UnormSrgb,
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

        let sampler = device.create_sampler(&SamplerDescriptor {
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Nearest,
            ..Default::default()
        });

        Self {
            bind_group_layout,
            pipeline,
            sampler,
        }
    }
}

/// A vertex in the UI.
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
struct Vertex {
    position: [f32; 3],
    uv: [f32; 2],
    color: [f32; 4],
}

impl Vertex {
    fn desc<'a>() -> VertexBufferLayout<'a> {
        VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as BufferAddress,
            step_mode: VertexStepMode::Vertex,
            attributes: &[
                VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: VertexFormat::Float32x3,
                },
                VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as BufferAddress,
                    shader_location: 1,
                    format: VertexFormat::Float32x2,
                },
                VertexAttribute {
                    offset: (std::mem::size_of::<[f32; 3]>() + std::mem::size_of::<[f32; 2]>())
                        as BufferAddress,
                    shader_location: 2,
                    format: VertexFormat::Float32x4,
                },
            ],
        }
    }
}

#[derive(Debug)]
pub struct PrimitiveElement {
    vertices: Buffer,
    indices: Buffer,
    texture: Texture,
    num_vertices: u32,
    bind_group: BindGroup,
}

impl PrimitiveElement {
    pub fn new(
        pipeline: &UiPipeline,
        device: &Device,
        queue: &Queue,
        start: Vec2,
        end: Vec2,
        image: &ImageBuffer<Rgba<u8>, Vec<u8>>,
        color: [f32; 4],
    ) -> Self {
        let vertices = [
            Vertex {
                position: [start.x, start.y, 0.0],
                uv: [0.0, 0.0],
                color,
            },
            Vertex {
                position: [start.x, end.y, 0.0],
                uv: [0.0, 1.0],
                color,
            },
            Vertex {
                position: [end.x, end.y, 0.0],
                uv: [1.0, 1.0],
                color,
            },
            Vertex {
                position: [end.x, start.y, 0.0],
                uv: [1.0, 0.0],
                color,
            },
        ];
        let indices: [u32; 6] = [0, 1, 2, 3, 0, 2];

        let num_vertices = indices.len() as u32;

        let vertices = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("primitive_element_vertex_buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: BufferUsages::VERTEX,
        });

        let indices = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("primitive_element_index_buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: BufferUsages::INDEX,
        });

        let texture = device.create_texture(&TextureDescriptor {
            label: Some("primitive_element_texture"),
            size: Extent3d {
                width: image.width(),
                height: image.height(),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8UnormSrgb,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            &image,
            ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * image.width()),
                rows_per_image: Some(image.height()),
            },
            Extent3d {
                width: image.width(),
                height: image.height(),
                depth_or_array_layers: 1,
            },
        );

        let texture_view = texture.create_view(&TextureViewDescriptor::default());

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("primitive_element_bind_group"),
            layout: &pipeline.bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&texture_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&pipeline.sampler),
                },
            ],
        });

        Self {
            vertices,
            indices,
            num_vertices,
            texture,
            bind_group,
        }
    }
}

#[derive(Debug)]
pub struct UiPass {
    pub elements: Vec<PrimitiveElement>,
}

impl UiPass {
    pub fn new() -> Self {
        Self { elements: vec![] }
    }

    pub fn update(
        &mut self,
        pipeline: &UiPipeline,
        device: &Device,
        queue: &Queue,
        size: Vec2,
        frame: &mut Frame,
    ) {
        // We don't need to update any elements if no elements
        // changed.
        if !frame.is_changed() {
            return;
        }

        frame.compute_layout();

        self.elements.clear();
        for (elem, layout) in frame.elements().zip(frame.layouts()) {
            self.elements
                .push(elem.build(*layout, pipeline, device, queue, size));
        }

        frame.unchanged();
    }

    pub fn render(&self, pipeline: &UiPipeline, ctx: RenderContext<'_>) {
        let mut render_pass = ctx.encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("ui_pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &ctx.view,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Load,
                    store: true,
                },
            })],
            depth_stencil_attachment: None,
        });

        render_pass.set_pipeline(&pipeline.pipeline);

        for elem in &self.elements {
            // let texture_view = elem.texture.create_view(&TextureViewDescriptor::default());

            // let bind_group = ctx.device.create_bind_group(&BindGroupDescriptor {
            //     layout: &pipeline.bind_group_layout,
            //     label: Some("ui_element_bind_group"),
            //     entries: &[
            //         BindGroupEntry {
            //             binding: 0,
            //             resource: BindingResource::TextureView(&texture_view),
            //         },
            //         BindGroupEntry {
            //             binding: 1,
            //             resource: BindingResource::Sampler(&pipeline.sampler),
            //         },
            //     ],
            // });

            render_pass.set_bind_group(0, &elem.bind_group, &[]);
            render_pass.set_vertex_buffer(0, elem.vertices.slice(..));
            render_pass.set_index_buffer(elem.indices.slice(..), IndexFormat::Uint32);

            render_pass.draw_indexed(0..elem.num_vertices, 0, 0..1);
        }

        // render_pass.set_bind_group(0, &self.bind_group, &[]);

        // render_pass.set_vertex_buffer(0, self.vertices.slice(..));
        // render_pass.set_index_buffer(self.indices.slice(..), IndexFormat::Uint32);

        // render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
    }
}

pub struct RenderContext<'a> {
    pub encoder: &'a mut CommandEncoder,
    pub view: &'a TextureView,
    pub device: &'a Device,
}

impl<'a> RenderContext<'a> {
    pub fn surface_view(&self) -> &TextureView {
        &self.view
    }

    // ICE
    // pub fn begin_render_pass(&'a mut self, desc: &RenderPassDescriptor<'a>) -> RenderPass<'a> {
    //     self.encoder.begin_render_pass(desc)
    // }
}

pub(crate) trait BuildPrimitiveElement {
    /// `size`: Screen size
    fn build(
        &self,
        layout: Layout,
        pipeline: &UiPipeline,
        device: &Device,
        queue: &Queue,
        size: Vec2,
    ) -> PrimitiveElement;
}

pub trait UiElement {}
