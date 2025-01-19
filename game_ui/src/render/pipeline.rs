use std::collections::HashMap;
use std::num::NonZeroU32;
use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use game_render::api::{
    BindingResource, Buffer, BufferInitDescriptor, CommandQueue, DepthStencilAttachment,
    DescriptorSetDescriptor, DescriptorSetEntry, DescriptorSetLayout,
    DescriptorSetLayoutDescriptor, Pipeline, PipelineDescriptor, RenderPassColorAttachment,
    RenderPassDescriptor, Sampler, Texture,
};
use game_render::backend::{
    AddressMode, BufferUsage, DescriptorBinding, DescriptorType, Face, FilterMode, FragmentStage,
    FrontFace, ImageDataLayout, IndexFormat, LoadOp, PipelineStage, PrimitiveTopology,
    SamplerDescriptor, ShaderModule, ShaderModuleDescriptor, ShaderSource, ShaderStage,
    ShaderStages, StoreOp, TextureDescriptor, TextureFormat, TextureUsage, VertexStage,
};
use game_render::camera::RenderTarget;
use game_render::graph::{Node, RenderContext, SlotLabel};
use game_tracing::trace_span;
use glam::UVec2;
use parking_lot::{Mutex, RwLock};

use super::remap::remap;
use super::{DrawCommand, GpuDrawCommandState, SurfaceDrawCommands};

const UI_SHADER: &str = include_str!("../../shaders/ui.wgsl");

/// The default texture array capacity.
const DEFAULT_TEXTURE_CAPACITY: NonZeroU32 = unsafe { NonZeroU32::new_unchecked(8) };

/// The factor at which the texture array capacity grows. Must be > 1.
const CAPACITY_GROWTH_FACTOR: NonZeroU32 = unsafe { NonZeroU32::new_unchecked(2) };

#[derive(Debug)]
struct UiPipeline {
    descriptor_set_layout: DescriptorSetLayout,
    sampler: Sampler,
    capacity: NonZeroU32,
    pipelines: HashMap<TextureFormat, Pipeline>,
    shader: ShaderModule,
}

impl UiPipeline {
    pub fn new(queue: &mut CommandQueue<'_>) -> Self {
        Self::new_with_capacity(queue, DEFAULT_TEXTURE_CAPACITY)
    }

    fn new_with_capacity(queue: &mut CommandQueue<'_>, capacity: NonZeroU32) -> Self {
        let descriptor_set_layout =
            queue.create_descriptor_set_layout(&DescriptorSetLayoutDescriptor {
                bindings: &[
                    DescriptorBinding {
                        binding: 0,
                        visibility: ShaderStages::VERTEX,
                        kind: DescriptorType::Storage,
                        count: NonZeroU32::MIN,
                    },
                    DescriptorBinding {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        kind: DescriptorType::Sampler,
                        count: NonZeroU32::MIN,
                    },
                    DescriptorBinding {
                        binding: 2,
                        visibility: ShaderStages::FRAGMENT,
                        kind: DescriptorType::Texture,
                        count: capacity,
                    },
                ],
            });

        let shader = queue.create_shader_module(ShaderSource::Wgsl(UI_SHADER));

        let sampler = queue.create_sampler(&SamplerDescriptor {
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Nearest,
        });

        Self {
            descriptor_set_layout,
            pipelines: HashMap::new(),
            sampler,
            capacity,
            shader,
        }
    }

    fn build_pipeline(&self, format: TextureFormat, queue: &mut CommandQueue<'_>) -> Pipeline {
        queue.create_pipeline(&PipelineDescriptor {
            topology: PrimitiveTopology::TriangleList,
            front_face: FrontFace::Ccw,
            cull_mode: Some(Face::Back),
            stages: &[
                PipelineStage::Vertex(VertexStage {
                    shader: &self.shader,
                    entry: "vs_main",
                }),
                PipelineStage::Fragment(FragmentStage {
                    shader: &self.shader,
                    entry: "fs_main",
                    targets: &[format],
                }),
            ],
            depth_stencil_state: None,
            descriptors: &[&self.descriptor_set_layout],
            push_constant_ranges: &[],
        })
    }
}

/// A vertex in the UI.
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
pub(super) struct Vertex {
    color: [f32; 4],
    position: [f32; 3],
    texture_index: u32,
    uv: [f32; 2],
    _pad0: [u32; 2],
}

#[derive(Debug)]
pub struct UiPass {
    pipeline: Mutex<UiPipeline>,
    elements: Arc<RwLock<HashMap<RenderTarget, SurfaceDrawCommands>>>,
    vertex_buffer: Mutex<Vec<u8>>,
    textures: Mutex<Vec<Texture>>,
    instance_count: Mutex<u32>,
    index_buffer: Buffer,
}

impl UiPass {
    pub(super) fn new(
        queue: &mut CommandQueue<'_>,
        elems: Arc<RwLock<HashMap<RenderTarget, SurfaceDrawCommands>>>,
    ) -> Self {
        let index_buffer = queue.create_buffer_init(&BufferInitDescriptor {
            contents: bytemuck::cast_slice(INDICES),
            usage: BufferUsage::INDEX,
        });

        Self {
            pipeline: Mutex::new(UiPipeline::new(queue)),
            elements: elems,
            vertex_buffer: Mutex::new(Vec::new()),
            instance_count: Mutex::new(0),
            index_buffer,
            textures: Mutex::new(Vec::new()),
        }
    }

    fn update_buffers(
        &self,
        target: RenderTarget,
        queue: &mut CommandQueue<'_>,
        viewport_size: UVec2,
    ) {
        let mut vertex_buffer = self.vertex_buffer.lock();
        let mut texture_buffer = self.textures.lock();
        let mut instance_count = self.instance_count.lock();

        vertex_buffer.clear();
        texture_buffer.clear();
        *instance_count = 0;

        let mut draw_cmds = self.elements.write();
        let Some(cmds) = draw_cmds.get_mut(&target) else {
            return;
        };

        for cmd in cmds.commands_mut() {
            let state = match &mut cmd.gpu_state {
                // For uploaded textures we must ensure that the texture size
                // matches the size of the current viewport, otherwise textures
                // will become squashed.
                // This can happen when the window is rapidly resized and the
                // ui state and renderer temporarily report different window sizes.
                Some(state) if state.size != viewport_size => {
                    let gpu_state = create_element(&cmd.cmd, viewport_size, queue);
                    cmd.gpu_state.insert(gpu_state)
                }
                None => {
                    let gpu_state = create_element(&cmd.cmd, viewport_size, queue);
                    cmd.gpu_state.insert(gpu_state)
                }
                Some(state) => state,
            };

            let texture_index = texture_buffer.len() as u32;
            texture_buffer.push(state.texture.clone());

            for vertex in &mut state.vertices {
                vertex.texture_index = texture_index;
            }

            vertex_buffer.extend(bytemuck::bytes_of(&state.vertices));

            *instance_count += 1;
        }
    }
}

impl Node for UiPass {
    fn render(&self, ctx: &mut RenderContext<'_, '_>) {
        let _span = trace_span!("UiPass::render").entered();

        self.update_buffers(ctx.render_target, ctx.queue, ctx.size);

        let mut pipeline = self.pipeline.lock();
        let vertex_buffer = self.vertex_buffer.lock();
        let texture_buffer = self.textures.lock();
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
            *pipeline = UiPipeline::new_with_capacity(&mut ctx.queue, new_capacity);
        }

        let vertex_buffer = ctx.queue.create_buffer_init(&BufferInitDescriptor {
            contents: &vertex_buffer,
            usage: BufferUsage::STORAGE,
        });

        let texture_views: Vec<&Texture> = texture_buffer.iter().collect();

        let bind_group = ctx.queue.create_descriptor_set(&DescriptorSetDescriptor {
            layout: &pipeline.descriptor_set_layout,
            entries: &[
                DescriptorSetEntry {
                    binding: 0,
                    resource: BindingResource::Buffer(&vertex_buffer),
                },
                DescriptorSetEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&pipeline.sampler),
                },
                DescriptorSetEntry {
                    binding: 2,
                    resource: BindingResource::TextureArray(&texture_views),
                },
            ],
        });

        // let indirect_buffer = ctx.queue.create_buffer_init(&BufferInitDescriptor {
        //     contents: DrawIndexedIndirectArgs {
        //         index_count: INDICES.len() as u32,
        //         instance_count,
        //         first_index: 0,
        //         base_vertex: 0,
        //         first_instance: 0,
        //     }
        //     .as_bytes(),
        //     usage: BufferUsage::INDIRECT,
        // });

        let render_pipeline = match pipeline.pipelines.get(&ctx.format) {
            Some(pl) => pl,
            None => {
                let pl = pipeline.build_pipeline(ctx.format, ctx.queue);
                pipeline.pipelines.insert(ctx.format, pl);
                pipeline.pipelines.get(&ctx.format).unwrap()
            }
        };

        let surface_texture = ctx.read::<Texture>(SlotLabel::SURFACE).unwrap().clone();
        let mut render_pass = ctx.queue.run_render_pass(&RenderPassDescriptor {
            color_attachments: &[RenderPassColorAttachment {
                texture: &surface_texture,
                load_op: LoadOp::Load,
                store_op: StoreOp::Store,
            }],
            depth_stencil_attachment: None,
        });

        render_pass.set_pipeline(&render_pipeline);
        render_pass.set_index_buffer(&self.index_buffer, IndexFormat::U16);
        render_pass.set_descriptor_set(0, &bind_group);
        // render_pass.draw_indexed_indirect(&indirect_buffer, 0);
        render_pass.draw_indexed(0..INDICES.len() as u32, 0, 0..instance_count);
    }
}

const INDICES: &[u16] = &[0, 1, 2, 3, 0, 2];

fn create_element(
    cmd: &DrawCommand,
    viewport_size: UVec2,
    queue: &mut CommandQueue<'_>,
) -> GpuDrawCommandState {
    let _span = trace_span!("create_element").entered();

    if cfg!(debug_assertions) && (cmd.image.height() == 0 || cmd.image.width() == 0) {
        panic!(
            "attempted to render a image with zero dimension x={}, y={}",
            cmd.image.width(),
            cmd.image.height()
        );
    }

    let texture = queue.create_texture(&TextureDescriptor {
        size: UVec2::new(cmd.image.width(), cmd.image.height()),
        mip_levels: 1,
        format: TextureFormat::Rgba8UnormSrgb,
        usage: TextureUsage::TEXTURE_BINDING | TextureUsage::TRANSFER_DST,
    });

    queue.write_texture(
        &texture,
        &cmd.image,
        ImageDataLayout {
            rows_per_image: 4 * cmd.image.width(),
            bytes_per_row: cmd.image.height(),
        },
    );

    let min = remap(cmd.position.min.as_vec2(), viewport_size.as_vec2());
    let max = remap(cmd.position.max.as_vec2(), viewport_size.as_vec2());

    let vertices = [
        Vertex {
            position: [min.x, min.y, 0.0],
            uv: [0.0, 0.0],
            color: cmd.color.as_rgba(),
            _pad0: [0; 2],
            texture_index: 0,
        },
        Vertex {
            position: [min.x, max.y, 0.0],
            uv: [0.0, 1.0],
            color: cmd.color.as_rgba(),
            _pad0: [0; 2],
            texture_index: 0,
        },
        Vertex {
            position: [max.x, max.y, 0.0],
            uv: [1.0, 1.0],
            color: cmd.color.as_rgba(),
            _pad0: [0; 2],
            texture_index: 0,
        },
        Vertex {
            position: [max.x, min.y, 0.0],
            uv: [1.0, 0.0],
            color: cmd.color.as_rgba(),
            _pad0: [0; 2],
            texture_index: 0,
        },
    ];

    GpuDrawCommandState {
        vertices,
        texture,
        size: viewport_size,
    }
}
