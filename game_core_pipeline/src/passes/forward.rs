use std::collections::HashMap;
use std::num::NonZeroU32;
use std::sync::Arc;

use game_common::components::Color;
use game_render::api::{
    BindingResource, BufferInitDescriptor, CommandQueue, DepthStencilAttachment,
    DescriptorSetDescriptor, DescriptorSetEntry, DescriptorSetLayout,
    DescriptorSetLayoutDescriptor, Pipeline, PipelineDescriptor, RenderPassColorAttachment,
    RenderPassDescriptor, Sampler, Texture, TextureViewDescriptor,
};
use game_render::backend::allocator::UsageFlags;
use game_render::backend::{
    AddressMode, BlendState, BufferUsage, ColorTargetState, CompareOp, DepthStencilState,
    DescriptorBinding, DescriptorType, FilterMode, FragmentStage, FrontFace, LoadOp, PipelineStage,
    PrimitiveTopology, PushConstantRange, SamplerDescriptor, ShaderModule, ShaderStages, StoreOp,
    TextureDescriptor, TextureFormat, TextureUsage, VertexStage,
};
use game_render::camera::RenderTarget;
use game_render::graph::{Node, RenderContext, SlotLabel};
use game_render::pipeline_cache::{PipelineBuilder, PipelineCache};
use game_render::shader::{ShaderConfig, ShaderLanguage, ShaderSource};
use game_tracing::trace_span;
use glam::UVec2;
use parking_lot::{Mutex, RwLock};

use crate::camera::{Camera, CameraUniform};

use super::{DEPTH_FORMAT, HDR_FORMAT, SceneData, State};

const VS_SHADER: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/shaders/forward_vs.wgsl");
const FS_SHADER: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/shaders/forward_fs.wgsl");

#[derive(Debug)]
pub(super) struct ForwardPass {
    state: Arc<Mutex<State>>,
    depth_stencils: RwLock<HashMap<RenderTarget, Texture>>,
    pipeline: PipelineCache<BuildForwardPipeline>,
    linear_sampler: Sampler,
    lights_and_sampler_descriptor: DescriptorSetLayout,
    hdr_texture_slot: SlotLabel,
}

impl ForwardPass {
    pub(super) fn new(
        queue: &mut CommandQueue<'_>,
        state: Arc<Mutex<State>>,
        hdr_texture_slot: SlotLabel,
    ) -> Self {
        let linear_sampler = queue.create_sampler(&SamplerDescriptor {
            address_mode_u: AddressMode::Repeat,
            address_mode_v: AddressMode::Repeat,
            address_mode_w: AddressMode::Repeat,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
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

        let pipeline;
        {
            let state = state.lock();
            pipeline = PipelineCache::new(
                BuildForwardPipeline {
                    mesh_descriptor: state.mesh_descriptor_layout.clone(),
                    model_descriptor: state.transform_descriptor_layout.clone(),
                    lights_and_sampler_descriptor: lights_and_sampler_descriptor.clone(),
                },
                vec![
                    ShaderConfig {
                        source: ShaderSource::File(VS_SHADER.into()),
                        language: ShaderLanguage::Wgsl,
                    },
                    ShaderConfig {
                        source: ShaderSource::File(FS_SHADER.into()),
                        language: ShaderLanguage::Wgsl,
                    },
                ],
            )
        }

        Self {
            state,
            depth_stencils: RwLock::new(HashMap::new()),
            pipeline,
            linear_sampler,
            lights_and_sampler_descriptor,
            hdr_texture_slot,
        }
    }

    fn update_depth_stencil(
        &self,
        queue: &mut CommandQueue<'_>,
        target: RenderTarget,
        size: UVec2,
    ) {
        let mut depth_stencils = self.depth_stencils.write();
        if let Some(texture) = depth_stencils.get(&target) {
            // Texture size unchanged, no need to recreate.
            if texture.size() == size {
                return;
            }
        }

        let texture = queue.create_texture(&TextureDescriptor {
            size,
            mip_levels: 1,
            format: DEPTH_FORMAT,
            usage: TextureUsage::RENDER_ATTACHMENT,
        });

        depth_stencils.insert(target, texture);
    }

    fn render_scene_with_camera(
        &self,
        ctx: &mut RenderContext<'_, '_>,
        state: &State,
        scene: &SceneData,
        camera: &Camera,
        size: UVec2,
    ) {
        let _span = trace_span!("ForwardPass::render_scene_with_camera").entered();

        let depth_stencils = self.depth_stencils.read();

        let textures = state.textures.views();
        let materials = ctx.queue.create_buffer_init(&BufferInitDescriptor {
            contents: state.material_slab.as_bytes(),
            usage: BufferUsage::STORAGE,
            flags: UsageFlags::empty(),
        });

        let lights_and_sampler_descriptor =
            ctx.queue.create_descriptor_set(&DescriptorSetDescriptor {
                layout: &self.lights_and_sampler_descriptor,
                entries: &[
                    DescriptorSetEntry {
                        binding: 0,
                        resource: BindingResource::Buffer(&scene.directional_light_buffer),
                    },
                    DescriptorSetEntry {
                        binding: 1,
                        resource: BindingResource::Buffer(&scene.point_light_buffer),
                    },
                    DescriptorSetEntry {
                        binding: 2,
                        resource: BindingResource::Buffer(&scene.spot_light_buffer),
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

        let pipeline = self.pipeline.get(ctx.queue, HDR_FORMAT);
        let depth_stencil = depth_stencils.get(&ctx.render_target).unwrap();
        let render_target = ctx.queue.create_texture(&TextureDescriptor {
            size,
            mip_levels: 1,
            format: HDR_FORMAT,
            usage: TextureUsage::TEXTURE_BINDING | TextureUsage::RENDER_ATTACHMENT,
        });

        let mut render_pass = ctx.queue.run_render_pass(&RenderPassDescriptor {
            color_attachments: &[RenderPassColorAttachment {
                target: &render_target.create_view(&TextureViewDescriptor::default()),
                load_op: LoadOp::Clear(Color::BLACK),
                store_op: StoreOp::Store,
            }],
            depth_stencil_attachment: Some(&DepthStencilAttachment {
                texture: depth_stencil,
                load_op: LoadOp::Clear(1.0),
                store_op: StoreOp::Store,
            }),
        });

        let mut push_constants = [0; 128];
        push_constants[0..80].copy_from_slice(bytemuck::bytes_of(&CameraUniform::new(
            camera.transform,
            camera.projection,
        )));

        render_pass.set_pipeline(&pipeline);
        render_pass.set_push_constants(
            ShaderStages::VERTEX | ShaderStages::FRAGMENT,
            0,
            &push_constants,
        );

        render_pass.set_descriptor_set(2, &lights_and_sampler_descriptor);

        for object in scene.objects.values() {
            let mesh = state.meshes.get(&object.mesh).unwrap();

            render_pass.set_descriptor_set(0, &object.transform);
            render_pass.set_descriptor_set(1, &mesh.descriptor);

            render_pass.set_index_buffer(&mesh.indices.buffer, mesh.indices.format);
            render_pass.draw_indexed(0..mesh.indices.len, 0, 0..1);
        }

        drop(render_pass);
        ctx.write(self.hdr_texture_slot, render_target).unwrap();
    }

    fn clear_pass(&self, ctx: &mut RenderContext<'_, '_>) {
        let texture = ctx.queue.create_texture(&TextureDescriptor {
            size: UVec2::ONE,
            mip_levels: 1,
            format: HDR_FORMAT,
            usage: TextureUsage::TEXTURE_BINDING | TextureUsage::RENDER_ATTACHMENT,
        });

        ctx.queue.run_render_pass(&RenderPassDescriptor {
            color_attachments: &[RenderPassColorAttachment {
                target: &texture.create_view(&TextureViewDescriptor::default()),
                load_op: LoadOp::Clear(Color::BLACK),
                store_op: StoreOp::Store,
            }],
            depth_stencil_attachment: None,
        });

        ctx.write(self.hdr_texture_slot, texture).unwrap();
    }
}

impl Node for ForwardPass {
    fn render<'a>(&self, ctx: &'a mut RenderContext<'_, 'a>) {
        let _span = trace_span!("ForwardPass::render").entered();

        let size = ctx.read::<Texture>(SlotLabel::SURFACE).unwrap().size();

        let state = self.state.lock();
        for scene in state.scenes.values() {
            for camera in scene.cameras.values() {
                if camera.target == ctx.render_target {
                    self.update_depth_stencil(ctx.queue, ctx.render_target, size);
                    self.render_scene_with_camera(ctx, &state, scene, camera, size);
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
struct BuildForwardPipeline {
    mesh_descriptor: DescriptorSetLayout,
    model_descriptor: DescriptorSetLayout,
    lights_and_sampler_descriptor: DescriptorSetLayout,
}

impl PipelineBuilder for BuildForwardPipeline {
    fn build(
        &self,
        queue: &mut CommandQueue<'_>,
        shaders: &[ShaderModule],
        format: TextureFormat,
    ) -> Pipeline {
        queue.create_pipeline(&PipelineDescriptor {
            topology: PrimitiveTopology::TriangleList,
            cull_mode: None,
            front_face: FrontFace::Ccw,
            descriptors: &[
                &self.model_descriptor,
                &self.mesh_descriptor,
                &self.lights_and_sampler_descriptor,
            ],
            stages: &[
                PipelineStage::Vertex(VertexStage {
                    shader: &shaders[0],
                    entry: "vs_main",
                }),
                PipelineStage::Fragment(FragmentStage {
                    shader: &shaders[1],
                    entry: "fs_main",
                    targets: &[ColorTargetState {
                        format,
                        blend: Some(BlendState::PREMULTIPLIED_ALPHA),
                    }],
                }),
            ],
            depth_stencil_state: Some(DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare_op: CompareOp::Less,
            }),
            push_constant_ranges: &[PushConstantRange {
                range: 0..128,
                stages: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
            }],
        })
    }
}
