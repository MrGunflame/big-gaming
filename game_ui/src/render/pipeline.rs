use std::collections::HashMap;
use std::num::NonZeroU32;
use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use game_render::camera::RenderTarget;
use game_render::graph::{Node, RenderContext};
use game_tracing::trace_span;
use glam::UVec2;
use parking_lot::{Mutex, RwLock};
use wgpu::util::{BufferInitDescriptor, DeviceExt, DrawIndexedIndirectArgs};
use wgpu::{
    AddressMode, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, BlendState, Buffer, BufferBindingType,
    BufferUsages, ColorTargetState, ColorWrites, Device, Extent3d, Face, FilterMode, FragmentState,
    FrontFace, ImageCopyTexture, ImageDataLayout, IndexFormat, LoadOp, MultisampleState,
    Operations, Origin3d, PipelineLayoutDescriptor, PolygonMode, PrimitiveState, PrimitiveTopology,
    Queue, RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline,
    RenderPipelineDescriptor, Sampler, SamplerBindingType, SamplerDescriptor,
    ShaderModuleDescriptor, ShaderSource, ShaderStages, StoreOp, TextureAspect, TextureDescriptor,
    TextureDimension, TextureFormat, TextureSampleType, TextureUsages, TextureView,
    TextureViewDescriptor, TextureViewDimension, VertexState,
};

use super::remap::remap;
use super::DrawCommand;

const UI_SHADER: &str = include_str!("../../shaders/ui.wgsl");

/// The default texture array capacity.
const DEFAULT_TEXTURE_CAPACITY: NonZeroU32 = unsafe { NonZeroU32::new_unchecked(8) };

/// The factor at which the texture array capacity grows. Must be > 1.
const CAPACITY_GROWTH_FACTOR: NonZeroU32 = unsafe { NonZeroU32::new_unchecked(2) };

#[derive(Debug)]
struct UiPipeline {
    bind_group_layout: BindGroupLayout,
    pipeline: RenderPipeline,
    sampler: Sampler,
    capacity: NonZeroU32,
}

impl UiPipeline {
    pub fn new(device: &Device) -> Self {
        Self::new_with_capacity(device, DEFAULT_TEXTURE_CAPACITY)
    }

    fn new_with_capacity(device: &Device, capacity: NonZeroU32) -> Self {
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("ui_layout"),
            entries: &[
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
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: Some(capacity),
                },
                BindGroupLayoutEntry {
                    binding: 2,
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
            source: ShaderSource::Wgsl(UI_SHADER.into()),
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("ui_pipeline"),
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
            capacity,
        }
    }
}

/// A vertex in the UI.
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
struct Vertex {
    color: [f32; 4],
    position: [f32; 3],
    texture_index: u32,
    uv: [f32; 2],
    _pad0: [u32; 2],
}

#[derive(Debug)]
pub struct UiPass {
    pipeline: Mutex<UiPipeline>,
    elements: Arc<RwLock<HashMap<RenderTarget, Vec<DrawCommand>>>>,
    vertex_buffer: Mutex<Vec<u8>>,
    texture_buffer: Mutex<Vec<TextureView>>,
    instance_count: Mutex<u32>,
    index_buffer: Buffer,
}

impl UiPass {
    pub(super) fn new(
        device: &Device,
        elems: Arc<RwLock<HashMap<RenderTarget, Vec<DrawCommand>>>>,
    ) -> Self {
        let index_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(INDICES),
            usage: BufferUsages::INDEX,
        });

        Self {
            pipeline: Mutex::new(UiPipeline::new(device)),
            elements: elems,
            vertex_buffer: Mutex::new(Vec::new()),
            texture_buffer: Mutex::new(Vec::new()),
            instance_count: Mutex::new(0),
            index_buffer,
        }
    }

    fn update_buffers(
        &self,
        target: RenderTarget,
        device: &Device,
        queue: &Queue,
        viewport_size: UVec2,
    ) {
        let mut vertex_buffer = self.vertex_buffer.lock();
        let mut texture_buffer = self.texture_buffer.lock();
        let mut instance_count = self.instance_count.lock();

        vertex_buffer.clear();
        texture_buffer.clear();
        *instance_count = 0;

        let draw_cmds = self.elements.read();
        let Some(cmds) = draw_cmds.get(&target) else {
            return;
        };

        for cmd in cmds {
            create_element(
                cmd,
                viewport_size,
                &mut vertex_buffer,
                &mut texture_buffer,
                device,
                queue,
            );
            *instance_count += 1;
        }
    }
}

impl Node for UiPass {
    fn render(&self, ctx: &mut RenderContext<'_>) {
        let _span = trace_span!("UiPass::render").entered();

        self.update_buffers(ctx.render_target, ctx.device, ctx.queue, ctx.size);

        let mut pipeline = self.pipeline.lock();
        let vertex_buffer = self.vertex_buffer.lock();
        let texture_buffer = self.texture_buffer.lock();
        let instance_count = *self.instance_count.lock();

        if instance_count == 0 {
            return;
        }

        // We have to recreate the pipeline with increased capacity if we have
        // more textures than we can store with the current pipeline layout.
        if texture_buffer.len() as u32 > pipeline.capacity.get() {
            let mut new_capacity = pipeline.capacity;
            while new_capacity.get() < texture_buffer.len() as u32 {
                new_capacity = match new_capacity.checked_mul(CAPACITY_GROWTH_FACTOR) {
                    Some(v) => v,
                    None => {
                        // FIXME: This case is pretty much unreachable because we will
                        // probably run out of VRAM before we reach u32::MAX, but we
                        // should handle this properly anyways.
                        // We can, for example split the render pass into multiple passes
                        // that render u32::MAX instances each.
                        panic!("UI texture limit reached");
                    }
                };
            }

            tracing::debug!("recreating UiPipeline with capacity {}", new_capacity);
            *pipeline = UiPipeline::new_with_capacity(&ctx.device, new_capacity);
        }

        let vertex_buffer = ctx.device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: &vertex_buffer,
            usage: BufferUsages::STORAGE,
        });

        let texture_views: Vec<&TextureView> = texture_buffer.iter().collect();

        let bind_group = ctx.device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &pipeline.bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: vertex_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureViewArray(&texture_views),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Sampler(&pipeline.sampler),
                },
            ],
        });

        let indirect_buffer = ctx.device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: DrawIndexedIndirectArgs {
                index_count: INDICES.len() as u32,
                instance_count,
                first_index: 0,
                base_vertex: 0,
                first_instance: 0,
            }
            .as_bytes(),
            usage: BufferUsages::INDIRECT,
        });

        let mut render_pass = ctx.encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("ui_pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: ctx.target,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Load,
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        render_pass.set_pipeline(&pipeline.pipeline);
        render_pass.set_index_buffer(self.index_buffer.slice(..), IndexFormat::Uint16);
        render_pass.set_bind_group(0, &bind_group, &[]);
        render_pass.draw_indexed_indirect(&indirect_buffer, 0);
    }
}

const INDICES: &[u16] = &[0, 1, 2, 3, 0, 2];

fn create_element(
    cmd: &DrawCommand,
    viewport_size: UVec2,
    vertex_buffer: &mut Vec<u8>,
    texture_buffer: &mut Vec<TextureView>,
    device: &Device,
    queue: &Queue,
) {
    let _span = trace_span!("create_element").entered();

    if cfg!(debug_assertions) && (cmd.image.height() == 0 || cmd.image.width() == 0) {
        panic!(
            "attempted to render a image with zero dimension x={}, y={}",
            cmd.image.width(),
            cmd.image.height()
        );
    }

    let texture = device.create_texture(&TextureDescriptor {
        label: None,
        size: Extent3d {
            width: cmd.image.width(),
            height: cmd.image.height(),
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
        &cmd.image,
        ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(4 * cmd.image.width()),
            rows_per_image: Some(cmd.image.height()),
        },
        Extent3d {
            width: cmd.image.width(),
            height: cmd.image.height(),
            depth_or_array_layers: 1,
        },
    );

    let texture_view = texture.create_view(&TextureViewDescriptor::default());
    let texture_index = texture_buffer.len() as u32;
    texture_buffer.push(texture_view);

    let min = remap(cmd.position.min.as_vec2(), viewport_size.as_vec2());
    let max = remap(cmd.position.max.as_vec2(), viewport_size.as_vec2());

    let vertices = [
        Vertex {
            position: [min.x, min.y, 0.0],
            uv: [0.0, 0.0],
            color: cmd.color.as_rgba(),
            _pad0: [0; 2],
            texture_index,
        },
        Vertex {
            position: [min.x, max.y, 0.0],
            uv: [0.0, 1.0],
            color: cmd.color.as_rgba(),
            _pad0: [0; 2],
            texture_index,
        },
        Vertex {
            position: [max.x, max.y, 0.0],
            uv: [1.0, 1.0],
            color: cmd.color.as_rgba(),
            _pad0: [0; 2],
            texture_index,
        },
        Vertex {
            position: [max.x, min.y, 0.0],
            uv: [1.0, 0.0],
            color: cmd.color.as_rgba(),
            _pad0: [0; 2],
            texture_index,
        },
    ];

    vertex_buffer.extend(bytemuck::cast_slice(&vertices));
}
