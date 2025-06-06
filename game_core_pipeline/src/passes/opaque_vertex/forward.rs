use std::num::NonZeroU32;
use std::sync::Arc;

use game_common::components::Color;
use game_render::api::{
    BindingResource, Buffer, CommandQueue, DepthStencilAttachment, DescriptorSetDescriptor,
    DescriptorSetEntry, DescriptorSetLayout, DescriptorSetLayoutDescriptor, Pipeline,
    PipelineDescriptor, RenderPassColorAttachment, RenderPassDescriptor, Sampler, Texture,
    TextureViewDescriptor,
};
use game_render::backend::{
    AddressMode, ColorTargetState, CompareOp, DepthStencilState, DescriptorBinding, DescriptorType,
    FilterMode, FragmentStage, FrontFace, IndexFormat, LoadOp, PipelineStage, PrimitiveTopology,
    PushConstantRange, SamplerDescriptor, ShaderStages, StoreOp, TextureDescriptor, TextureFormat,
    TextureUsage, VertexStage,
};
use game_render::graph::{Node, RenderContext, SlotLabel};
use game_render::pipeline_cache::{PipelineBuilder, PipelineCache};
use game_render::shader::{Shader, ShaderConfig, ShaderLanguage, ShaderSource};
use game_tracing::trace_span;
use glam::UVec2;
use parking_lot::Mutex;

use crate::camera::CameraUniform;
use crate::entities::{CameraId, SceneId};
use crate::passes::{DEPTH_FORMAT, HDR_FORMAT, HDR_TEXTURE, MeshStateImpl, State};

use super::{INDIRECT_DRAW_BUFFER, INSTANCE_BUFFER};

const VS_SHADER: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/shaders/opaque_vertex/forward_vert.slang"
);
const FS_SHADER: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/shaders/forward_frag.slang");

#[derive(Debug)]
pub struct OpaqueVertexForwardPass {
    state: Arc<Mutex<State>>,
    pipeline: PipelineCache<BuildPipeline>,
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

        let mesh_descriptor = queue.create_descriptor_set_layout(&DescriptorSetLayoutDescriptor {
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
        });

        let lights_and_sampler_descriptor =
            queue.create_descriptor_set_layout(&DescriptorSetLayoutDescriptor {
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
            });

        let pipeline = PipelineCache::new(
            BuildPipeline {
                mesh_descriptor,
                lights_and_sampler_descriptor,
            },
            vec![
                ShaderConfig {
                    source: ShaderSource::File(VS_SHADER.into()),
                    language: ShaderLanguage::Slang,
                },
                ShaderConfig {
                    source: ShaderSource::File(FS_SHADER.into()),
                    language: ShaderLanguage::Slang,
                },
            ],
        );

        Self {
            state,
            pipeline,
            linear_sampler,
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
            self.clear_pass(ctx);
            return;
        };

        let detpth_texture = ctx.queue.create_texture(&TextureDescriptor {
            size,
            mip_levels: 1,
            format: DEPTH_FORMAT,
            usage: TextureUsage::RENDER_ATTACHMENT,
        });

        let scene = state.scenes.get_mut(&scene).unwrap();
        let camera = scene.cameras.get(&camera).unwrap();

        let textures = state.textures.views();
        let materials = state.material_slab.buffer(ctx.queue);

        let directional_lights = scene.directional_lights_buffer.buffer(ctx.queue);
        let point_lights = scene.point_lights_buffer.buffer(ctx.queue);
        let spot_lights = scene.spot_lights_buffer.buffer(ctx.queue);

        let lights_and_sampler_descriptor =
            ctx.queue.create_descriptor_set(&DescriptorSetDescriptor {
                layout: &self.pipeline.builder.lights_and_sampler_descriptor,
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

        let MeshStateImpl::Vertex(mesh_state) = &mut state.mesh else {
            unreachable!();
        };

        let offsets = mesh_state.mesh_offsets.buffer(ctx.queue);

        let mesh_descriptor = ctx.queue.create_descriptor_set(&DescriptorSetDescriptor {
            layout: &self.pipeline.builder.mesh_descriptor,
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
                    resource: BindingResource::Buffer(instance_buffer),
                },
            ],
        });

        let pipeline = self.pipeline.get(ctx.queue, HDR_FORMAT);
        let render_target = ctx.queue.create_texture(&TextureDescriptor {
            size,
            mip_levels: 1,
            format: HDR_FORMAT,
            usage: TextureUsage::TEXTURE_BINDING | TextureUsage::RENDER_ATTACHMENT,
        });

        let mut render_pass = ctx.queue.run_render_pass(&RenderPassDescriptor {
            name: "Forward",
            color_attachments: &[RenderPassColorAttachment {
                target: &render_target.create_view(&TextureViewDescriptor::default()),
                load_op: LoadOp::Clear(Color::BLACK),
                store_op: StoreOp::Store,
            }],
            depth_stencil_attachment: Some(&DepthStencilAttachment {
                texture: &detpth_texture,
                load_op: LoadOp::Clear(0.0),
                store_op: StoreOp::Store,
            }),
        });

        let mut push_constants = [0; 128];
        push_constants[0..80].copy_from_slice(bytemuck::bytes_of(&CameraUniform::new(
            camera.transform,
            camera.projection,
        )));
        push_constants[80..84]
            .copy_from_slice(&(scene.directional_lights.len() as u32).to_ne_bytes());
        push_constants[84..88].copy_from_slice(&(scene.spot_lights.len() as u32).to_ne_bytes());
        push_constants[88..92].copy_from_slice(&(scene.spot_lights.len() as u32).to_ne_bytes());

        render_pass.set_pipeline(&pipeline);
        render_pass.set_push_constants(
            ShaderStages::VERTEX | ShaderStages::FRAGMENT,
            0,
            &push_constants,
        );

        render_pass.set_descriptor_set(0, &mesh_descriptor);
        render_pass.set_descriptor_set(1, &lights_and_sampler_descriptor);
        render_pass.set_index_buffer(mesh_state.index_buffer.buffer(), IndexFormat::U32);
        render_pass.draw_indexed_indirect(indirect_buffer);

        drop(render_pass);
        ctx.write(HDR_TEXTURE, render_target).unwrap();
    }

    fn clear_pass(&self, ctx: &mut RenderContext<'_, '_>) {
        let texture = ctx.queue.create_texture(&TextureDescriptor {
            size: UVec2::ONE,
            mip_levels: 1,
            format: HDR_FORMAT,
            usage: TextureUsage::TEXTURE_BINDING | TextureUsage::RENDER_ATTACHMENT,
        });

        ctx.queue.run_render_pass(&RenderPassDescriptor {
            name: "Forward",
            color_attachments: &[RenderPassColorAttachment {
                target: &texture.create_view(&TextureViewDescriptor::default()),
                load_op: LoadOp::Clear(Color::BLACK),
                store_op: StoreOp::Store,
            }],
            depth_stencil_attachment: None,
        });

        ctx.write(HDR_TEXTURE, texture).unwrap();
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
        self.clear_pass(ctx);
    }
}

#[derive(Debug)]
struct BuildPipeline {
    mesh_descriptor: DescriptorSetLayout,
    lights_and_sampler_descriptor: DescriptorSetLayout,
}

impl PipelineBuilder for BuildPipeline {
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
            descriptors: &[&self.mesh_descriptor, &self.lights_and_sampler_descriptor],
            stages: &[
                PipelineStage::Vertex(VertexStage {
                    shader: &shaders[0],
                    entry: "main",
                }),
                PipelineStage::Fragment(FragmentStage {
                    shader: &shaders[1],
                    entry: "main",
                    targets: &[ColorTargetState {
                        format: HDR_FORMAT,
                        blend: None,
                    }],
                }),
            ],
            depth_stencil_state: Some(DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare_op: CompareOp::Greater,
            }),
            push_constant_ranges: &[PushConstantRange {
                range: 0..128,
                stages: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
            }],
        })
    }
}
