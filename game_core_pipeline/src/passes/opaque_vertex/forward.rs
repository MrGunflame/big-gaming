use std::num::NonZeroU32;
use std::sync::Arc;

use game_common::components::Color;
use game_render::api::{
    BindingResource, Buffer, BufferDescriptor, CommandQueue, ComputePassDescriptor,
    DepthStencilAttachment, DescriptorSetDescriptor, DescriptorSetEntry, DescriptorSetLayout,
    DescriptorSetLayoutDescriptor, Pipeline, PipelineDescriptor, RenderPassColorAttachment,
    RenderPassDescriptor, Sampler, Texture, TextureViewDescriptor,
};
use game_render::backend::allocator::UsageFlags;
use game_render::backend::{
    AddressMode, BufferUsage, ColorTargetState, CompareOp, ComputeStage, DepthStencilState,
    DescriptorBinding, DescriptorType, DrawIndirectCommand, Face, FilterMode, FragmentStage,
    FrontFace, LoadOp, PipelineStage, PrimitiveTopology, PushConstantRange, SamplerDescriptor,
    ShaderStages, StoreOp, TextureDescriptor, TextureFormat, TextureUsage, VertexStage,
};
use game_render::graph::{Node, RenderContext, SlotLabel};
use game_render::pipeline_cache::{PipelineBuilder, PipelineCache};
use game_render::shader::{Shader, ShaderConfig, ShaderLanguage, ShaderSource};
use game_tracing::trace_span;
use glam::UVec2;
use parking_lot::Mutex;

use crate::camera::CameraUniform;
use crate::entities::{CameraId, SceneId};
use crate::passes::opaque_vertex::state::Instance;
use crate::passes::{DEPTH_FORMAT, HDR_FORMAT, HDR_TEXTURE, MeshStateImpl, OPAQUE_DEPTH, State};

use super::{INDIRECT_DRAW_BUFFER, INSTANCE_BUFFER};

const FORWARD_VS_SHADER: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/shaders/opaque_vertex/visbuffer_vert.slang"
);
const FORWARD_FS_SHADER: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/shaders/opaque_vertex/visbuffer_frag.slang"
);
const SHADING_SHADER: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/shaders/opaque_vertex/visbuffer_shading.slang"
);
const ALPHA_MASK_VS_SHADER: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/shaders/opaque_vertex/transparent_mask_vert.slang"
);
const ALPHA_MASK_FS_SHADER: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/shaders/opaque_vertex/transparent_mask_frag.slang"
);

const DRAWCALL_GEN_SHADER: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/shaders/opaque_vertex/drawcall_gen.slang"
);

const WORKGROUP_SIZE: u32 = 64;

#[derive(Debug)]
pub struct OpaqueVertexForwardPass {
    state: Arc<Mutex<State>>,
    drawcall_gen_pipeline: PipelineCache<BuildDrawcallGenPipeline>,
    forward_pipeline: PipelineCache<ForwardPipelineBuilder>,
    alpha_mask_pipeline: PipelineCache<AlphaMaskPipelineBuilder>,
    shading_pipeline: PipelineCache<ShadingPipelineBuilder>,
    linear_sampler: Sampler,
}

impl OpaqueVertexForwardPass {
    pub fn new(queue: &CommandQueue<'_>, state: Arc<Mutex<State>>) -> Self {
        let linear_sampler = queue.create_sampler(&SamplerDescriptor {
            address_mode_u: AddressMode::Repeat,
            address_mode_v: AddressMode::Repeat,
            address_mode_w: AddressMode::Repeat,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
        });

        let drawcall_gen_pipeline = PipelineCache::new(
            BuildDrawcallGenPipeline {
                descriptor: queue.create_descriptor_set_layout(&DescriptorSetLayoutDescriptor {
                    bindings: &[
                        DescriptorBinding {
                            binding: 0,
                            visibility: ShaderStages::COMPUTE,
                            kind: DescriptorType::Storage,
                            count: NonZeroU32::MIN,
                        },
                        DescriptorBinding {
                            binding: 1,
                            visibility: ShaderStages::COMPUTE,
                            kind: DescriptorType::Storage,
                            count: NonZeroU32::MIN,
                        },
                        DescriptorBinding {
                            binding: 2,
                            visibility: ShaderStages::COMPUTE,
                            kind: DescriptorType::Storage,
                            count: NonZeroU32::MIN,
                        },
                    ],
                }),
            },
            vec![ShaderConfig {
                language: ShaderLanguage::Slang,
                source: ShaderSource::File(DRAWCALL_GEN_SHADER.into()),
            }],
        );

        let forward_pipeline = PipelineCache::new(
            ForwardPipelineBuilder {
                mesh_descriptor: queue.create_descriptor_set_layout(
                    &DescriptorSetLayoutDescriptor {
                        bindings: &[
                            DescriptorBinding {
                                binding: 0,
                                visibility: ShaderStages::VERTEX,
                                kind: DescriptorType::Storage,
                                count: NonZeroU32::MIN,
                            },
                            DescriptorBinding {
                                binding: 1,
                                visibility: ShaderStages::VERTEX,
                                kind: DescriptorType::Storage,
                                count: NonZeroU32::MIN,
                            },
                            DescriptorBinding {
                                binding: 2,
                                visibility: ShaderStages::VERTEX,
                                kind: DescriptorType::Storage,
                                count: NonZeroU32::MIN,
                            },
                            DescriptorBinding {
                                binding: 3,
                                visibility: ShaderStages::VERTEX,
                                kind: DescriptorType::Storage,
                                count: NonZeroU32::MIN,
                            },
                        ],
                    },
                ),
            },
            vec![
                ShaderConfig {
                    source: ShaderSource::File(FORWARD_VS_SHADER.into()),
                    language: ShaderLanguage::Slang,
                },
                ShaderConfig {
                    source: ShaderSource::File(FORWARD_FS_SHADER.into()),
                    language: ShaderLanguage::Slang,
                },
            ],
        );

        let alpha_mask_pipeline = PipelineCache::new(
            AlphaMaskPipelineBuilder {
                descriptor: queue.create_descriptor_set_layout(&DescriptorSetLayoutDescriptor {
                    bindings: &[
                        DescriptorBinding {
                            binding: 0,
                            visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                            kind: DescriptorType::Storage,
                            count: NonZeroU32::MIN,
                        },
                        DescriptorBinding {
                            binding: 1,
                            visibility: ShaderStages::VERTEX,
                            kind: DescriptorType::Storage,
                            count: NonZeroU32::MIN,
                        },
                        DescriptorBinding {
                            binding: 2,
                            visibility: ShaderStages::VERTEX,
                            kind: DescriptorType::Storage,
                            count: NonZeroU32::MIN,
                        },
                        DescriptorBinding {
                            binding: 3,
                            visibility: ShaderStages::VERTEX,
                            kind: DescriptorType::Storage,
                            count: NonZeroU32::MIN,
                        },
                        DescriptorBinding {
                            binding: 4,
                            visibility: ShaderStages::VERTEX,
                            kind: DescriptorType::Storage,
                            count: NonZeroU32::MIN,
                        },
                        DescriptorBinding {
                            binding: 5,
                            visibility: ShaderStages::VERTEX,
                            kind: DescriptorType::Storage,
                            count: NonZeroU32::MIN,
                        },
                        DescriptorBinding {
                            binding: 6,
                            visibility: ShaderStages::FRAGMENT,
                            kind: DescriptorType::Storage,
                            count: NonZeroU32::MIN,
                        },
                        DescriptorBinding {
                            binding: 7,
                            visibility: ShaderStages::FRAGMENT,
                            kind: DescriptorType::Texture,
                            // FIXME: Unhardcode this
                            count: NonZeroU32::new(8192).unwrap(),
                        },
                        DescriptorBinding {
                            binding: 8,
                            visibility: ShaderStages::FRAGMENT,
                            kind: DescriptorType::Sampler,
                            count: NonZeroU32::MIN,
                        },
                    ],
                }),
            },
            vec![
                ShaderConfig {
                    language: ShaderLanguage::Slang,
                    source: ShaderSource::File(ALPHA_MASK_VS_SHADER.into()),
                },
                ShaderConfig {
                    language: ShaderLanguage::Slang,
                    source: ShaderSource::File(ALPHA_MASK_FS_SHADER.into()),
                },
            ],
        );

        let shading_pipeline = PipelineCache::new(
            ShadingPipelineBuilder {
                io_descriptor: queue.create_descriptor_set_layout(&DescriptorSetLayoutDescriptor {
                    bindings: &[
                        DescriptorBinding {
                            binding: 0,
                            visibility: ShaderStages::COMPUTE,
                            kind: DescriptorType::Texture,
                            count: NonZeroU32::MIN,
                        },
                        DescriptorBinding {
                            binding: 1,
                            visibility: ShaderStages::COMPUTE,
                            kind: DescriptorType::StorageTexture,
                            count: NonZeroU32::MIN,
                        },
                    ],
                }),
                mesh_descriptor: queue.create_descriptor_set_layout(
                    &DescriptorSetLayoutDescriptor {
                        bindings: &[
                            DescriptorBinding {
                                binding: 0,
                                visibility: ShaderStages::COMPUTE,
                                kind: DescriptorType::Storage,
                                count: NonZeroU32::MIN,
                            },
                            DescriptorBinding {
                                binding: 1,
                                visibility: ShaderStages::COMPUTE,
                                kind: DescriptorType::Storage,
                                count: NonZeroU32::MIN,
                            },
                            DescriptorBinding {
                                binding: 2,
                                visibility: ShaderStages::COMPUTE,
                                kind: DescriptorType::Storage,
                                count: NonZeroU32::MIN,
                            },
                            DescriptorBinding {
                                binding: 3,
                                visibility: ShaderStages::COMPUTE,
                                kind: DescriptorType::Storage,
                                count: NonZeroU32::MIN,
                            },
                            DescriptorBinding {
                                binding: 4,
                                visibility: ShaderStages::COMPUTE,
                                kind: DescriptorType::Storage,
                                count: NonZeroU32::MIN,
                            },
                            DescriptorBinding {
                                binding: 5,
                                visibility: ShaderStages::COMPUTE,
                                kind: DescriptorType::Storage,
                                count: NonZeroU32::MIN,
                            },
                            DescriptorBinding {
                                binding: 6,
                                visibility: ShaderStages::COMPUTE,
                                kind: DescriptorType::Storage,
                                count: NonZeroU32::MIN,
                            },
                            DescriptorBinding {
                                binding: 7,
                                visibility: ShaderStages::COMPUTE,
                                kind: DescriptorType::Storage,
                                count: NonZeroU32::MIN,
                            },
                            DescriptorBinding {
                                binding: 8,
                                visibility: ShaderStages::COMPUTE,
                                kind: DescriptorType::Storage,
                                count: NonZeroU32::MIN,
                            },
                        ],
                    },
                ),
                shading_descriptor: queue.create_descriptor_set_layout(
                    &DescriptorSetLayoutDescriptor {
                        bindings: &[
                            DescriptorBinding {
                                binding: 0,
                                visibility: ShaderStages::COMPUTE,
                                kind: DescriptorType::Storage,
                                count: NonZeroU32::MIN,
                            },
                            DescriptorBinding {
                                binding: 1,
                                visibility: ShaderStages::COMPUTE,
                                kind: DescriptorType::Storage,
                                count: NonZeroU32::MIN,
                            },
                            DescriptorBinding {
                                binding: 2,
                                visibility: ShaderStages::COMPUTE,
                                kind: DescriptorType::Storage,
                                count: NonZeroU32::MIN,
                            },
                            DescriptorBinding {
                                binding: 3,
                                visibility: ShaderStages::COMPUTE,
                                kind: DescriptorType::Storage,
                                count: NonZeroU32::MIN,
                            },
                            DescriptorBinding {
                                binding: 4,
                                visibility: ShaderStages::COMPUTE,
                                kind: DescriptorType::Texture,
                                // TODO: Grow when needed instead of initializing this to a value.
                                count: NonZeroU32::new(8192).unwrap(),
                            },
                            DescriptorBinding {
                                binding: 5,
                                visibility: ShaderStages::COMPUTE,
                                kind: DescriptorType::Sampler,
                                count: NonZeroU32::MIN,
                            },
                        ],
                    },
                ),
            },
            vec![ShaderConfig {
                source: ShaderSource::File(SHADING_SHADER.into()),
                language: ShaderLanguage::Slang,
            }],
        );

        Self {
            state,
            linear_sampler,
            forward_pipeline,
            shading_pipeline,
            alpha_mask_pipeline,
            drawcall_gen_pipeline,
        }
    }

    fn render_scene_with_camera(
        &self,
        ctx: &mut RenderContext<'_, '_>,
        state: &mut State,
        scene: SceneId,
        camera: CameraId,
        size: UVec2,
    ) {
        let _span = trace_span!("ForwardPass::render_scene_with_camera").entered();

        let (Ok(instance_buffer), Ok(indirect_buffer)) = (
            ctx.read::<Buffer>(INSTANCE_BUFFER),
            ctx.read::<Buffer>(INDIRECT_DRAW_BUFFER),
        ) else {
            self.clear_pass(ctx, size);
            return;
        };

        let depth_texture = ctx.queue.create_texture(&TextureDescriptor {
            size,
            mip_levels: 1,
            format: DEPTH_FORMAT,
            usage: TextureUsage::RENDER_ATTACHMENT | TextureUsage::TEXTURE_BINDING,
        });

        let scene = state.scenes.get_mut(&scene).unwrap();
        let camera = scene.cameras.get(&camera).unwrap();

        let textures = state.textures.views();
        let materials = state.material_slab.buffer(ctx.queue);

        let directional_lights = scene.directional_lights_buffer.buffer(ctx.queue);
        let point_lights = scene.point_lights_buffer.buffer(ctx.queue);
        let spot_lights = scene.spot_lights_buffer.buffer(ctx.queue);

        let MeshStateImpl::Vertex(mesh_state) = &mut state.mesh else {
            unreachable!();
        };

        let positions = mesh_state.positions.buffer();
        let normals = mesh_state.normals.buffer();
        let tangents = mesh_state.tangents.buffer();
        let uvs = mesh_state.uvs.buffer();
        let colors = mesh_state.colors.buffer();

        let offsets = mesh_state.mesh_offsets.buffer(ctx.queue);
        let index_buffer = mesh_state.index_buffer.buffer();

        let visbuffer = ctx.queue.create_texture(&TextureDescriptor {
            size,
            mip_levels: 1,
            format: TextureFormat::Rg32Uint,
            usage: TextureUsage::RENDER_ATTACHMENT | TextureUsage::TEXTURE_BINDING,
        });

        {
            let mesh_descriptor = ctx.queue.create_descriptor_set(&DescriptorSetDescriptor {
                layout: &self.forward_pipeline.builder.mesh_descriptor,
                entries: &[
                    DescriptorSetEntry {
                        binding: 0,
                        resource: BindingResource::Buffer(positions),
                    },
                    DescriptorSetEntry {
                        binding: 1,
                        resource: BindingResource::Buffer(offsets),
                    },
                    DescriptorSetEntry {
                        binding: 2,
                        resource: BindingResource::Buffer(instance_buffer),
                    },
                    DescriptorSetEntry {
                        binding: 3,
                        resource: BindingResource::Buffer(index_buffer),
                    },
                ],
            });

            let pipeline = self
                .forward_pipeline
                .get(ctx.queue, TextureFormat::Rg32Uint);

            let mut pass = ctx.queue.run_render_pass(&RenderPassDescriptor {
                name: "Forward",
                color_attachments: &[RenderPassColorAttachment {
                    load_op: LoadOp::Clear(Color::BLACK),
                    store_op: StoreOp::Store,
                    target: &visbuffer.create_view(&TextureViewDescriptor::default()),
                }],
                depth_stencil_attachment: Some(&DepthStencilAttachment {
                    texture: &depth_texture,
                    load_op: LoadOp::Clear(0.0),
                    store_op: StoreOp::Store,
                }),
            });

            pass.set_pipeline(&pipeline);
            pass.set_descriptor_set(0, &mesh_descriptor);

            let mut push_constants = [0; 128];
            push_constants[0..80].copy_from_slice(bytemuck::bytes_of(&CameraUniform::new(
                camera.transform,
                camera.projection,
            )));
            push_constants[80..84]
                .copy_from_slice(&(scene.directional_lights.len() as u32).to_ne_bytes());
            push_constants[84..88]
                .copy_from_slice(&(scene.point_lights.len() as u32).to_ne_bytes());
            push_constants[88..92].copy_from_slice(&(scene.spot_lights.len() as u32).to_ne_bytes());

            pass.set_push_constants(
                ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                0,
                &push_constants,
            );

            pass.draw_indirect(indirect_buffer);
        }

        let alpha_mask_instances = ctx.queue.create_buffer(&BufferDescriptor {
            size: mesh_state
                .transparent_mask_instances
                .buffer(ctx.queue)
                .size()
                .max(size_of::<Instance>() as u64),
            flags: UsageFlags::empty(),
            usage: BufferUsage::STORAGE,
        });

        // Alpha Mask forward
        if mesh_state.num_transparent_mask_instances != 0 {
            let instance_in = mesh_state.transparent_mask_instances.buffer(ctx.queue);

            let draws = ctx.queue.create_buffer(&BufferDescriptor {
                size: mesh_state.num_transparent_mask_instances as u64
                    * size_of::<DrawIndirectCommand>() as u64,
                usage: BufferUsage::STORAGE | BufferUsage::INDIRECT,
                flags: UsageFlags::empty(),
            });

            {
                let pipeline = self.drawcall_gen_pipeline.get(ctx.queue, HDR_FORMAT);

                let descriptor = ctx.queue.create_descriptor_set(&DescriptorSetDescriptor {
                    layout: &self.drawcall_gen_pipeline.builder.descriptor,
                    entries: &[
                        DescriptorSetEntry {
                            binding: 0,
                            resource: BindingResource::Buffer(instance_in),
                        },
                        DescriptorSetEntry {
                            binding: 1,
                            resource: BindingResource::Buffer(&alpha_mask_instances),
                        },
                        DescriptorSetEntry {
                            binding: 2,
                            resource: BindingResource::Buffer(&draws),
                        },
                    ],
                });

                let mut compute_pass = ctx.queue.run_compute_pass(&ComputePassDescriptor {
                    name: "Drawcall Gen (Transparent Mask)",
                });

                let workgroups = mesh_state
                    .num_transparent_mask_instances
                    .div_ceil(WORKGROUP_SIZE);

                compute_pass.set_pipeline(&pipeline);
                compute_pass.set_push_constants(
                    ShaderStages::COMPUTE,
                    0,
                    bytemuck::bytes_of(&mesh_state.num_transparent_mask_instances),
                );
                compute_pass.set_descriptor_set(0, &descriptor);
                compute_pass.dispatch(workgroups, 1, 1);
            }

            let descriptor = ctx.queue.create_descriptor_set(&DescriptorSetDescriptor {
                layout: &self.alpha_mask_pipeline.builder.descriptor,
                entries: &[
                    DescriptorSetEntry {
                        binding: 0,
                        resource: BindingResource::Buffer(&alpha_mask_instances),
                    },
                    DescriptorSetEntry {
                        binding: 1,
                        resource: BindingResource::Buffer(&offsets),
                    },
                    DescriptorSetEntry {
                        binding: 2,
                        resource: BindingResource::Buffer(index_buffer),
                    },
                    DescriptorSetEntry {
                        binding: 3,
                        resource: BindingResource::Buffer(positions),
                    },
                    DescriptorSetEntry {
                        binding: 4,
                        resource: BindingResource::Buffer(uvs),
                    },
                    DescriptorSetEntry {
                        binding: 5,
                        resource: BindingResource::Buffer(colors),
                    },
                    DescriptorSetEntry {
                        binding: 6,
                        resource: BindingResource::Buffer(materials),
                    },
                    DescriptorSetEntry {
                        binding: 7,
                        resource: BindingResource::TextureArray(&textures),
                    },
                    DescriptorSetEntry {
                        binding: 8,
                        resource: BindingResource::Sampler(&self.linear_sampler),
                    },
                ],
            });

            let pipeline = self.alpha_mask_pipeline.get(ctx.queue, HDR_FORMAT);

            let mut pass = ctx.queue.run_render_pass(&RenderPassDescriptor {
                name: "Alpha Mask Forward",
                color_attachments: &[RenderPassColorAttachment {
                    load_op: LoadOp::Load,
                    store_op: StoreOp::Store,
                    target: &visbuffer.create_view(&TextureViewDescriptor::default()),
                }],
                depth_stencil_attachment: Some(&DepthStencilAttachment {
                    texture: &depth_texture,
                    load_op: LoadOp::Load,
                    store_op: StoreOp::Store,
                }),
            });

            pass.set_pipeline(&pipeline);
            pass.set_descriptor_set(0, &descriptor);

            let mut push_constants = [0; 128];
            push_constants[0..80].copy_from_slice(bytemuck::bytes_of(&CameraUniform::new(
                camera.transform,
                camera.projection,
            )));

            pass.set_push_constants(ShaderStages::VERTEX, 0, &push_constants);

            pass.draw_indirect(&draws);
        }

        let output_texture = ctx.queue.create_texture(&TextureDescriptor {
            size,
            mip_levels: 1,
            format: HDR_FORMAT,
            usage: TextureUsage::TEXTURE_BINDING
                | TextureUsage::RENDER_ATTACHMENT
                | TextureUsage::STORAGE,
        });

        {
            let io_descriptor = ctx.queue.create_descriptor_set(&DescriptorSetDescriptor {
                layout: &self.shading_pipeline.builder.io_descriptor,
                entries: &[
                    DescriptorSetEntry {
                        binding: 0,
                        resource: BindingResource::Texture(
                            &visbuffer.create_view(&TextureViewDescriptor::default()),
                        ),
                    },
                    DescriptorSetEntry {
                        binding: 1,
                        resource: BindingResource::Texture(
                            &output_texture.create_view(&TextureViewDescriptor::default()),
                        ),
                    },
                ],
            });

            let mesh_descriptor = ctx.queue.create_descriptor_set(&DescriptorSetDescriptor {
                layout: &self.shading_pipeline.builder.mesh_descriptor,
                entries: &[
                    DescriptorSetEntry {
                        binding: 0,
                        resource: BindingResource::Buffer(positions),
                    },
                    DescriptorSetEntry {
                        binding: 1,
                        resource: BindingResource::Buffer(normals),
                    },
                    DescriptorSetEntry {
                        binding: 2,
                        resource: BindingResource::Buffer(uvs),
                    },
                    DescriptorSetEntry {
                        binding: 3,
                        resource: BindingResource::Buffer(tangents),
                    },
                    DescriptorSetEntry {
                        binding: 4,
                        resource: BindingResource::Buffer(colors),
                    },
                    DescriptorSetEntry {
                        binding: 5,
                        resource: BindingResource::Buffer(offsets),
                    },
                    DescriptorSetEntry {
                        binding: 6,
                        resource: BindingResource::Buffer(instance_buffer),
                    },
                    DescriptorSetEntry {
                        binding: 7,
                        resource: BindingResource::Buffer(index_buffer),
                    },
                    DescriptorSetEntry {
                        binding: 8,
                        resource: BindingResource::Buffer(&alpha_mask_instances),
                    },
                ],
            });

            let shading_descriptor = ctx.queue.create_descriptor_set(&DescriptorSetDescriptor {
                layout: &self.shading_pipeline.builder.shading_descriptor,
                entries: &[
                    DescriptorSetEntry {
                        binding: 0,
                        resource: BindingResource::Buffer(directional_lights),
                    },
                    DescriptorSetEntry {
                        binding: 1,
                        resource: BindingResource::Buffer(point_lights),
                    },
                    DescriptorSetEntry {
                        binding: 2,
                        resource: BindingResource::Buffer(spot_lights),
                    },
                    DescriptorSetEntry {
                        binding: 3,
                        resource: BindingResource::Buffer(materials),
                    },
                    DescriptorSetEntry {
                        binding: 4,
                        resource: BindingResource::TextureArray(&textures),
                    },
                    DescriptorSetEntry {
                        binding: 5,
                        resource: BindingResource::Sampler(&self.linear_sampler),
                    },
                ],
            });

            let pipeline = self.shading_pipeline.get(ctx.queue, TextureFormat::R32Uint);
            let mut pass = ctx.queue.run_compute_pass(&ComputePassDescriptor {
                name: "Visbuffer Shading",
            });

            pass.set_pipeline(&pipeline);
            pass.set_descriptor_set(0, &io_descriptor);
            pass.set_descriptor_set(1, &mesh_descriptor);
            pass.set_descriptor_set(2, &shading_descriptor);

            let mut push_constants = [0; 128];
            push_constants[0..80].copy_from_slice(bytemuck::bytes_of(&CameraUniform::new(
                camera.transform,
                camera.projection,
            )));
            push_constants[80..84]
                .copy_from_slice(&(scene.directional_lights.len() as u32).to_ne_bytes());
            push_constants[84..88]
                .copy_from_slice(&(scene.point_lights.len() as u32).to_ne_bytes());
            push_constants[88..92].copy_from_slice(&(scene.spot_lights.len() as u32).to_ne_bytes());
            push_constants[96..104].copy_from_slice(bytemuck::bytes_of(&size));

            pass.set_push_constants(ShaderStages::COMPUTE, 0, &push_constants);

            let x = size.x.div_ceil(16);
            let y = size.y.div_ceil(16);
            pass.dispatch(x, y, 1);
        }

        ctx.write(HDR_TEXTURE, output_texture).unwrap();
        ctx.write(OPAQUE_DEPTH, depth_texture).unwrap();
    }

    fn clear_pass(&self, ctx: &mut RenderContext<'_, '_>, size: UVec2) {
        let texture = ctx.queue.create_texture(&TextureDescriptor {
            size,
            mip_levels: 1,
            format: HDR_FORMAT,
            usage: TextureUsage::TEXTURE_BINDING
                | TextureUsage::RENDER_ATTACHMENT
                | TextureUsage::STORAGE,
        });

        let depth_texture = ctx.queue.create_texture(&TextureDescriptor {
            size,
            mip_levels: 1,
            format: DEPTH_FORMAT,
            usage: TextureUsage::TEXTURE_BINDING | TextureUsage::RENDER_ATTACHMENT,
        });

        ctx.queue.run_render_pass(&RenderPassDescriptor {
            name: "Forward",
            color_attachments: &[RenderPassColorAttachment {
                target: &texture.create_view(&TextureViewDescriptor::default()),
                load_op: LoadOp::Clear(Color::BLACK),
                store_op: StoreOp::Store,
            }],
            depth_stencil_attachment: Some(&DepthStencilAttachment {
                texture: &depth_texture,
                load_op: LoadOp::Clear(0.0),
                store_op: StoreOp::Store,
            }),
        });

        ctx.write(HDR_TEXTURE, texture).unwrap();
        ctx.write(OPAQUE_DEPTH, depth_texture).unwrap();
    }
}

impl Node for OpaqueVertexForwardPass {
    fn render<'a>(&self, ctx: &'a mut RenderContext<'_, 'a>) {
        let _span = trace_span!("OpaqueVertexForwardPass::render").entered();

        let size = ctx.read::<Texture>(SlotLabel::SURFACE).unwrap().size();

        let mut state = self.state.lock();
        for (scene_id, scene) in state.scenes.iter() {
            for (camera_id, camera) in scene.cameras.iter() {
                if camera.target == ctx.render_target {
                    let scene_id = *scene_id;
                    let camera_id = *camera_id;
                    self.render_scene_with_camera(ctx, &mut state, scene_id, camera_id, size);
                    return;
                }
            }
        }

        // If we don't have any camera to render we just
        // emit a black texture.
        self.clear_pass(ctx, size);
    }
}

#[derive(Debug)]
struct ForwardPipelineBuilder {
    mesh_descriptor: DescriptorSetLayout,
}

impl PipelineBuilder for ForwardPipelineBuilder {
    fn build(
        &self,
        queue: &CommandQueue<'_>,
        shaders: &[Shader],
        _format: TextureFormat,
    ) -> Pipeline {
        queue.create_pipeline(&PipelineDescriptor {
            topology: PrimitiveTopology::TriangleList,
            front_face: FrontFace::Ccw,
            cull_mode: Some(Face::Back),
            stages: &[
                PipelineStage::Vertex(VertexStage {
                    shader: &shaders[0],
                    entry: "main",
                }),
                PipelineStage::Fragment(FragmentStage {
                    shader: &shaders[1],
                    entry: "main",
                    targets: &[ColorTargetState {
                        format: TextureFormat::Rg32Uint,
                        blend: None,
                    }],
                }),
            ],
            descriptors: &[&self.mesh_descriptor],
            push_constant_ranges: &[PushConstantRange {
                range: 0..128,
                stages: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
            }],
            depth_stencil_state: Some(DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare_op: CompareOp::Greater,
            }),
        })
    }
}

#[derive(Debug)]
struct ShadingPipelineBuilder {
    io_descriptor: DescriptorSetLayout,
    mesh_descriptor: DescriptorSetLayout,
    shading_descriptor: DescriptorSetLayout,
}

impl PipelineBuilder for ShadingPipelineBuilder {
    fn build(
        &self,
        queue: &CommandQueue<'_>,
        shaders: &[Shader],
        _format: TextureFormat,
    ) -> Pipeline {
        queue.create_pipeline(&PipelineDescriptor {
            topology: PrimitiveTopology::TriangleList,
            front_face: FrontFace::Ccw,
            cull_mode: None,
            stages: &[PipelineStage::Compute(ComputeStage {
                shader: &shaders[0],
                entry: "main",
            })],
            descriptors: &[
                &self.io_descriptor,
                &self.mesh_descriptor,
                &self.shading_descriptor,
            ],
            push_constant_ranges: &[PushConstantRange {
                range: 0..128,
                stages: ShaderStages::COMPUTE,
            }],
            depth_stencil_state: None,
        })
    }
}

#[derive(Debug)]
struct AlphaMaskPipelineBuilder {
    descriptor: DescriptorSetLayout,
}

impl PipelineBuilder for AlphaMaskPipelineBuilder {
    fn build(
        &self,
        queue: &CommandQueue<'_>,
        shaders: &[Shader],
        _format: TextureFormat,
    ) -> Pipeline {
        queue.create_pipeline(&PipelineDescriptor {
            topology: PrimitiveTopology::TriangleList,
            front_face: FrontFace::Ccw,
            cull_mode: Some(Face::Back),
            stages: &[
                PipelineStage::Vertex(VertexStage {
                    shader: &shaders[0],
                    entry: "main",
                }),
                PipelineStage::Fragment(FragmentStage {
                    shader: &shaders[1],
                    entry: "main",
                    targets: &[ColorTargetState {
                        format: TextureFormat::Rg32Uint,
                        blend: None,
                    }],
                }),
            ],
            descriptors: &[&self.descriptor],
            push_constant_ranges: &[PushConstantRange {
                range: 0..128,
                stages: ShaderStages::VERTEX,
            }],
            depth_stencil_state: Some(DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare_op: CompareOp::Greater,
            }),
        })
    }
}

#[derive(Debug)]
struct BuildDrawcallGenPipeline {
    descriptor: DescriptorSetLayout,
}

impl PipelineBuilder for BuildDrawcallGenPipeline {
    fn build(
        &self,
        queue: &CommandQueue<'_>,
        shaders: &[Shader],
        _format: TextureFormat,
    ) -> Pipeline {
        queue.create_pipeline(&PipelineDescriptor {
            topology: PrimitiveTopology::TriangleList,
            cull_mode: None,
            front_face: FrontFace::Ccw,
            descriptors: &[&self.descriptor],
            stages: &[PipelineStage::Compute(ComputeStage {
                shader: &shaders[0],
                entry: "main",
            })],
            depth_stencil_state: None,
            push_constant_ranges: &[PushConstantRange {
                range: 0..4,
                stages: ShaderStages::COMPUTE,
            }],
        })
    }
}
