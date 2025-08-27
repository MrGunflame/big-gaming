//! Vertex pipeline based transparency pass

use std::num::NonZeroU32;
use std::sync::Arc;

use game_render::api::{
    BindingResource, BufferDescriptor, BufferInitDescriptor, CommandQueue, ComputePassDescriptor,
    DescriptorSetDescriptor, DescriptorSetEntry, DescriptorSetLayout,
    DescriptorSetLayoutDescriptor, Pipeline, PipelineDescriptor, RenderPassColorAttachment,
    RenderPassDescriptor, Sampler, Texture, TextureRegion, TextureViewDescriptor,
};
use game_render::backend::allocator::UsageFlags;
use game_render::backend::{
    AddressMode, BufferUsage, CompareOp, ComputeStage, DepthStencilState, DescriptorBinding,
    DescriptorType, DrawIndexedIndirectCommand, Face, FilterMode, FragmentStage, FrontFace,
    IndexFormat, LoadOp, PipelineStage, PrimitiveTopology, PushConstantRange, SamplerDescriptor,
    ShaderStages, StoreOp, TextureDescriptor, TextureFormat, TextureUsage, VertexStage,
};
use game_render::graph::{Node, RenderContext};
use game_render::pipeline_cache::{PipelineBuilder, PipelineCache};
use game_render::shader::{Shader, ShaderConfig, ShaderLanguage, ShaderSource};
use game_tracing::trace_span;
use parking_lot::Mutex;

use crate::camera::CameraUniform;
use crate::entities::{CameraId, SceneId};
use crate::passes::{DEPTH_FORMAT, HDR_FORMAT, HDR_TEXTURE, MeshStateImpl, OPAQUE_DEPTH, State};

const DRAWGEN_SHADER: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/shaders/transparent/drawcall_gen.slang"
);
const COLOR_VERT_SHADER: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/shaders/transparent/transparent_vert.slang"
);
const COLOR_FRAG_SHADER: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/shaders/transparent/transparent_frag.slang"
);
const BLEND_SHADER: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/shaders/transparent/transparent_blend.slang"
);

const WORKGROUP_SIZE: u32 = 64;

#[derive(Debug)]
pub struct TransparentVertexPass {
    state: Arc<Mutex<State>>,
    drawcall_gen_pipeline: PipelineCache<DrawcallGenPipeline>,
    color_pipeline: PipelineCache<BuildColorPipeline>,
    blend_pipeline: PipelineCache<BuildBlendPipeline>,
    linear_sampler: Sampler,
}

impl TransparentVertexPass {
    pub fn new(state: Arc<Mutex<State>>, queue: &CommandQueue<'_>) -> Self {
        Self {
            state,
            drawcall_gen_pipeline: PipelineCache::new(
                DrawcallGenPipeline {
                    descriptor: queue.create_descriptor_set_layout(
                        &DescriptorSetLayoutDescriptor {
                            bindings: &[
                                // Instances in
                                DescriptorBinding {
                                    binding: 0,
                                    visibility: ShaderStages::COMPUTE,
                                    kind: DescriptorType::Storage,
                                    count: NonZeroU32::MIN,
                                },
                                // Instances out
                                DescriptorBinding {
                                    binding: 1,
                                    visibility: ShaderStages::COMPUTE,
                                    kind: DescriptorType::Storage,
                                    count: NonZeroU32::MIN,
                                },
                                // Draw commands
                                DescriptorBinding {
                                    binding: 2,
                                    visibility: ShaderStages::COMPUTE,
                                    kind: DescriptorType::Storage,
                                    count: NonZeroU32::MIN,
                                },
                            ],
                        },
                    ),
                },
                vec![ShaderConfig {
                    source: ShaderSource::File(DRAWGEN_SHADER.into()),
                    language: ShaderLanguage::Slang,
                }],
            ),
            color_pipeline: PipelineCache::new(
                BuildColorPipeline {
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
                                    visibility: ShaderStages::VERTEX,
                                    kind: DescriptorType::Storage,
                                    count: NonZeroU32::MIN,
                                },
                            ],
                        },
                    ),
                    material_descriptor: queue.create_descriptor_set_layout(
                        &DescriptorSetLayoutDescriptor {
                            bindings: &[
                                DescriptorBinding {
                                    binding: 0,
                                    visibility: ShaderStages::FRAGMENT,
                                    kind: DescriptorType::Storage,
                                    count: NonZeroU32::MIN,
                                },
                                DescriptorBinding {
                                    binding: 1,
                                    visibility: ShaderStages::FRAGMENT,
                                    kind: DescriptorType::Storage,
                                    count: NonZeroU32::MIN,
                                },
                                DescriptorBinding {
                                    binding: 2,
                                    visibility: ShaderStages::FRAGMENT,
                                    kind: DescriptorType::Storage,
                                    count: NonZeroU32::MIN,
                                },
                                DescriptorBinding {
                                    binding: 3,
                                    visibility: ShaderStages::FRAGMENT,
                                    kind: DescriptorType::Storage,
                                    count: NonZeroU32::MIN,
                                },
                                DescriptorBinding {
                                    binding: 4,
                                    visibility: ShaderStages::FRAGMENT,
                                    kind: DescriptorType::Texture,
                                    // TODO: Grow when needed instead of initializing this to a value.
                                    count: NonZeroU32::new(8192).unwrap(),
                                },
                                DescriptorBinding {
                                    binding: 5,
                                    visibility: ShaderStages::FRAGMENT,
                                    kind: DescriptorType::Sampler,
                                    count: NonZeroU32::MIN,
                                },
                            ],
                        },
                    ),
                    output_descriptor: queue.create_descriptor_set_layout(
                        &DescriptorSetLayoutDescriptor {
                            bindings: &[
                                // A-Buffer
                                DescriptorBinding {
                                    binding: 0,
                                    visibility: ShaderStages::FRAGMENT,
                                    kind: DescriptorType::Storage,
                                    count: NonZeroU32::MIN,
                                },
                                // Linked list heads
                                DescriptorBinding {
                                    binding: 1,
                                    visibility: ShaderStages::FRAGMENT,
                                    kind: DescriptorType::StorageTexture,
                                    count: NonZeroU32::MIN,
                                },
                                // Shared atomic counter
                                DescriptorBinding {
                                    binding: 2,
                                    visibility: ShaderStages::FRAGMENT,
                                    kind: DescriptorType::Storage,
                                    count: NonZeroU32::MIN,
                                },
                                // Depth buffer
                                DescriptorBinding {
                                    binding: 3,
                                    visibility: ShaderStages::FRAGMENT,
                                    kind: DescriptorType::Texture,
                                    count: NonZeroU32::MIN,
                                },
                            ],
                        },
                    ),
                },
                vec![
                    ShaderConfig {
                        source: ShaderSource::File(COLOR_VERT_SHADER.into()),
                        language: ShaderLanguage::Slang,
                    },
                    ShaderConfig {
                        source: ShaderSource::File(COLOR_FRAG_SHADER.into()),
                        language: ShaderLanguage::Slang,
                    },
                ],
            ),
            blend_pipeline: PipelineCache::new(
                BuildBlendPipeline {
                    descriptor: queue.create_descriptor_set_layout(
                        &DescriptorSetLayoutDescriptor {
                            bindings: &[
                                // A-Buffer
                                DescriptorBinding {
                                    binding: 0,
                                    visibility: ShaderStages::COMPUTE,
                                    kind: DescriptorType::Storage,
                                    count: NonZeroU32::MIN,
                                },
                                // Heads Buffer
                                DescriptorBinding {
                                    binding: 1,
                                    visibility: ShaderStages::COMPUTE,
                                    kind: DescriptorType::Texture,
                                    count: NonZeroU32::MIN,
                                },
                                // Output Texture
                                DescriptorBinding {
                                    binding: 2,
                                    visibility: ShaderStages::COMPUTE,
                                    kind: DescriptorType::StorageTexture,
                                    count: NonZeroU32::MIN,
                                },
                            ],
                        },
                    ),
                },
                vec![ShaderConfig {
                    source: ShaderSource::File(BLEND_SHADER.into()),
                    language: ShaderLanguage::Slang,
                }],
            ),
            linear_sampler: queue.create_sampler(&SamplerDescriptor {
                address_mode_u: AddressMode::Repeat,
                address_mode_v: AddressMode::Repeat,
                address_mode_w: AddressMode::Repeat,
                mag_filter: FilterMode::Linear,
                min_filter: FilterMode::Linear,
                mipmap_filter: FilterMode::Linear,
            }),
        }
    }
}

impl TransparentVertexPass {
    fn render_scene_with_camera(
        &self,
        ctx: &mut RenderContext<'_, '_>,
        state: &mut State,
        scene: SceneId,
        camera: CameraId,
    ) {
        let scene = state.scenes.get_mut(&scene).unwrap();
        let camera = scene.cameras.get(&camera).unwrap();

        let MeshStateImpl::Vertex(mesh_state) = &mut state.mesh else {
            unreachable!();
        };

        let depth_target = ctx.read::<Texture>(OPAQUE_DEPTH).unwrap();
        let color_target = ctx.read::<Texture>(HDR_TEXTURE).unwrap();
        let size = color_target.size();

        // Size for at least four layers if every pixel is covered.
        // The size of one element is 16 bytes.
        let abuffer_size = size.x as u64 * size.y as u64 * 4 * 16;

        if mesh_state.num_transparent_instances == 0 || size.x == 0 || size.y == 0 {
            return;
        }

        let instance_buffer;
        let indirect_buffer;

        // Drawcall Generation Pass
        {
            let instances_in = mesh_state.transparent_instances.buffer(ctx.queue);
            let instances_out = ctx.queue.create_buffer(&BufferDescriptor {
                size: instances_in.size(),
                usage: BufferUsage::STORAGE,
                flags: UsageFlags::empty(),
            });
            let draws = ctx.queue.create_buffer(&BufferDescriptor {
                size: mesh_state.num_transparent_instances as u64
                    * size_of::<DrawIndexedIndirectCommand>() as u64,
                usage: BufferUsage::STORAGE | BufferUsage::INDIRECT,
                flags: UsageFlags::empty(),
            });

            let descriptor = ctx.queue.create_descriptor_set(&DescriptorSetDescriptor {
                layout: &self.drawcall_gen_pipeline.builder.descriptor,
                entries: &[
                    DescriptorSetEntry {
                        binding: 0,
                        resource: BindingResource::Buffer(&instances_in),
                    },
                    DescriptorSetEntry {
                        binding: 1,
                        resource: BindingResource::Buffer(&instances_out),
                    },
                    DescriptorSetEntry {
                        binding: 2,
                        resource: BindingResource::Buffer(&draws),
                    },
                ],
            });

            let mut pass = ctx.queue.run_compute_pass(&ComputePassDescriptor {
                name: "Transparent Drawcall Generation Pass",
            });

            let num_dispatches = mesh_state
                .num_transparent_instances
                .div_ceil(WORKGROUP_SIZE);

            pass.set_pipeline(&self.drawcall_gen_pipeline.get(ctx.queue, HDR_FORMAT));
            pass.set_push_constants(
                ShaderStages::COMPUTE,
                0,
                bytemuck::bytes_of(&mesh_state.num_transparent_instances),
            );
            pass.set_descriptor_set(0, &descriptor);
            pass.dispatch(num_dispatches, 1, 1);

            instance_buffer = instances_out;
            indirect_buffer = draws;
        }

        let abuffer;
        let heads;

        // Color Pass
        {
            let offsets = mesh_state.mesh_offsets.buffer(ctx.queue);

            let mesh_descriptor = ctx.queue.create_descriptor_set(&DescriptorSetDescriptor {
                layout: &self.color_pipeline.builder.mesh_descriptor,
                entries: &[
                    DescriptorSetEntry {
                        binding: 0,
                        resource: BindingResource::Buffer(mesh_state.positions.buffer()),
                    },
                    DescriptorSetEntry {
                        binding: 1,
                        resource: BindingResource::Buffer(mesh_state.normals.buffer()),
                    },
                    DescriptorSetEntry {
                        binding: 2,
                        resource: BindingResource::Buffer(mesh_state.uvs.buffer()),
                    },
                    DescriptorSetEntry {
                        binding: 3,
                        resource: BindingResource::Buffer(mesh_state.tangents.buffer()),
                    },
                    DescriptorSetEntry {
                        binding: 4,
                        resource: BindingResource::Buffer(mesh_state.colors.buffer()),
                    },
                    DescriptorSetEntry {
                        binding: 5,
                        resource: BindingResource::Buffer(offsets),
                    },
                    DescriptorSetEntry {
                        binding: 6,
                        resource: BindingResource::Buffer(&instance_buffer),
                    },
                ],
            });

            let textures = state.textures.views();
            let materials = state.material_slab.buffer(ctx.queue);

            let directional_lights = scene.directional_lights_buffer.buffer(ctx.queue);
            let point_lights = scene.point_lights_buffer.buffer(ctx.queue);
            let spot_lights = scene.spot_lights_buffer.buffer(ctx.queue);

            let material_descriptor = ctx.queue.create_descriptor_set(&DescriptorSetDescriptor {
                layout: &self.color_pipeline.builder.material_descriptor,
                entries: &[
                    DescriptorSetEntry {
                        binding: 0,
                        resource: BindingResource::Buffer(&directional_lights),
                    },
                    DescriptorSetEntry {
                        binding: 1,
                        resource: BindingResource::Buffer(&point_lights),
                    },
                    DescriptorSetEntry {
                        binding: 2,
                        resource: BindingResource::Buffer(&spot_lights),
                    },
                    DescriptorSetEntry {
                        binding: 3,
                        resource: BindingResource::Buffer(&materials),
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

            abuffer = ctx.queue.create_buffer(&BufferDescriptor {
                size: abuffer_size,
                usage: BufferUsage::STORAGE,
                flags: UsageFlags::empty(),
            });
            heads = ctx.queue.create_texture(&TextureDescriptor {
                size: size,
                mip_levels: 1,
                format: TextureFormat::R32Uint,
                usage: TextureUsage::TRANSFER_DST
                    | TextureUsage::TEXTURE_BINDING
                    | TextureUsage::STORAGE,
            });

            // Zero out all values of `heads`.
            ctx.queue.clear_texture(
                TextureRegion {
                    texture: &heads,
                    mip_level: 0,
                },
                [0; 4],
            );

            let counter = ctx.queue.create_buffer_init(&BufferInitDescriptor {
                // We need to initialize the counter to 1 so that the
                // value 0 is never read. The value 0 indicates the end of
                // the linked list chain.
                contents: bytemuck::bytes_of(&1_u32),
                usage: BufferUsage::STORAGE,
                flags: UsageFlags::empty(),
            });

            let output_descriptor = ctx.queue.create_descriptor_set(&DescriptorSetDescriptor {
                layout: &self.color_pipeline.builder.output_descriptor,
                entries: &[
                    DescriptorSetEntry {
                        binding: 0,
                        resource: BindingResource::Buffer(&abuffer),
                    },
                    DescriptorSetEntry {
                        binding: 1,
                        resource: BindingResource::Texture(
                            &heads.create_view(&TextureViewDescriptor::default()),
                        ),
                    },
                    DescriptorSetEntry {
                        binding: 2,
                        resource: BindingResource::Buffer(&counter),
                    },
                    DescriptorSetEntry {
                        binding: 3,
                        resource: BindingResource::Texture(
                            &depth_target.create_view(&TextureViewDescriptor::default()),
                        ),
                    },
                ],
            });

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

            let mut pass = ctx.queue.run_render_pass(&RenderPassDescriptor {
                name: "Transparent (Vertex) Color Pass",
                color_attachments: &[RenderPassColorAttachment {
                    target: &color_target.create_view(&TextureViewDescriptor::default()),
                    load_op: LoadOp::Load,
                    store_op: StoreOp::Store,
                }],
                // We use the depth buffer from the opaque pass to reject
                // as many pixels as possible early.
                // depth_stencil_attachment: Some(&DepthStencilAttachment {
                //     texture: &depth_target,
                //     load_op: LoadOp::Load,
                //     store_op: StoreOp::Discard,
                // }),
                depth_stencil_attachment: None,
            });

            pass.set_pipeline(&self.color_pipeline.get(ctx.queue, HDR_FORMAT));
            pass.set_push_constants(
                ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                0,
                &push_constants,
            );
            pass.set_descriptor_set(0, &mesh_descriptor);
            pass.set_descriptor_set(1, &material_descriptor);
            pass.set_descriptor_set(2, &output_descriptor);
            pass.set_index_buffer(mesh_state.index_buffer.buffer(), IndexFormat::U32);
            pass.draw_indexed_indirect(&indirect_buffer);
        }

        // Blend Pass
        {
            let mut pass = ctx.queue.run_compute_pass(&ComputePassDescriptor {
                name: "Transparent Blend Pass",
            });

            let descriptor = ctx.queue.create_descriptor_set(&DescriptorSetDescriptor {
                layout: &self.blend_pipeline.builder.descriptor,
                entries: &[
                    DescriptorSetEntry {
                        binding: 0,
                        resource: BindingResource::Buffer(&abuffer),
                    },
                    DescriptorSetEntry {
                        binding: 1,
                        resource: BindingResource::Texture(
                            &heads.create_view(&TextureViewDescriptor::default()),
                        ),
                    },
                    DescriptorSetEntry {
                        binding: 2,
                        resource: BindingResource::Texture(
                            &color_target.create_view(&TextureViewDescriptor::default()),
                        ),
                    },
                ],
            });

            let num_dispatches = (size.x * size.y).div_ceil(WORKGROUP_SIZE);

            pass.set_pipeline(&self.blend_pipeline.get(ctx.queue, HDR_FORMAT));
            pass.set_push_constants(ShaderStages::COMPUTE, 0, bytemuck::bytes_of(&size));
            pass.set_descriptor_set(0, &descriptor);
            pass.dispatch(num_dispatches, 1, 1);
        }
    }
}

impl Node for TransparentVertexPass {
    fn render<'a>(&self, ctx: &'a mut RenderContext<'_, 'a>) {
        let _span = trace_span!("TransparentVertexPass::render").entered();

        let mut state = self.state.lock();
        for (scene_id, scene) in state.scenes.iter() {
            for (camera_id, camera) in scene.cameras.iter() {
                if camera.target == ctx.render_target {
                    let scene_id = *scene_id;
                    let camera_id = *camera_id;
                    self.render_scene_with_camera(ctx, &mut state, scene_id, camera_id);
                    return;
                }
            }
        }
    }
}

#[derive(Debug)]
struct DrawcallGenPipeline {
    descriptor: DescriptorSetLayout,
}

impl PipelineBuilder for DrawcallGenPipeline {
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
            descriptors: &[&self.descriptor],
            push_constant_ranges: &[PushConstantRange {
                range: 0..4,
                stages: ShaderStages::COMPUTE,
            }],
            depth_stencil_state: None,
        })
    }
}

#[derive(Debug)]
struct BuildColorPipeline {
    mesh_descriptor: DescriptorSetLayout,
    material_descriptor: DescriptorSetLayout,
    output_descriptor: DescriptorSetLayout,
}

impl PipelineBuilder for BuildColorPipeline {
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
                    targets: &[],
                }),
            ],
            descriptors: &[
                &self.mesh_descriptor,
                &self.material_descriptor,
                &self.output_descriptor,
            ],
            push_constant_ranges: &[PushConstantRange {
                range: 0..128,
                stages: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
            }],
            depth_stencil_state: Some(DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: false,
                depth_compare_op: CompareOp::Greater,
            }),
        })
    }
}

#[derive(Debug)]
struct BuildBlendPipeline {
    descriptor: DescriptorSetLayout,
}

impl PipelineBuilder for BuildBlendPipeline {
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
            descriptors: &[&self.descriptor],
            push_constant_ranges: &[PushConstantRange {
                range: 0..8,
                stages: ShaderStages::COMPUTE,
            }],
            depth_stencil_state: None,
        })
    }
}
