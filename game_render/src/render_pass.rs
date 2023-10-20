use std::collections::HashMap;
use std::sync::Arc;

use game_tracing::trace_span;
use glam::{Mat4, Vec3};
use parking_lot::Mutex;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, Buffer, BufferUsages, Color, Device, Extent3d,
    LoadOp, Operations, RenderPassColorAttachment, RenderPassDepthStencilAttachment,
    RenderPassDescriptor, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
    TextureViewDescriptor,
};

use crate::buffer::{DynamicBuffer, IndexBuffer};
use crate::camera::{CameraBuffer, RenderTarget, OPENGL_TO_WGPU};
use crate::entities::{CameraId, ObjectId};
use crate::forward::ForwardPipeline;
use crate::graph::{Node, RenderContext};
use crate::light::pipeline::{DirectionalLightUniform, PointLightUniform, SpotLightUniform};
use crate::light::DirectionalLight;
use crate::post_process::PostProcessPipeline;
use crate::shadow::ShadowPipeline;
use crate::state::RenderState;

pub struct GpuObject {
    pub indices: IndexBuffer,
    pub mesh_bind_group: BindGroup,
    pub material_bind_group: BindGroup,
    pub transform: Buffer,
}

pub struct GpuState {
    pub cameras: HashMap<CameraId, CameraBuffer>,
    pub objects: HashMap<ObjectId, GpuObject>,
    pub directional_lights: Buffer,
    pub point_lights: Buffer,
    pub spot_lights: Buffer,
}

impl GpuState {
    pub fn new(device: &Device) -> Self {
        let buffer = DynamicBuffer::<DirectionalLightUniform>::new();
        let directional_lights = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: buffer.as_bytes(),
            usage: BufferUsages::STORAGE,
        });

        let buffer = DynamicBuffer::<PointLightUniform>::new();
        let point_lights = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: buffer.as_bytes(),
            usage: BufferUsages::STORAGE,
        });

        let buffer = DynamicBuffer::<SpotLightUniform>::new();
        let spot_lights = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: buffer.as_bytes(),
            usage: BufferUsages::STORAGE,
        });

        Self {
            directional_lights,
            point_lights,
            spot_lights,
            objects: HashMap::new(),
            cameras: HashMap::new(),
        }
    }
}

pub(crate) struct RenderPass {
    pub state: Arc<Mutex<RenderState>>,
    pub forward: Arc<ForwardPipeline>,
    pub post_process: PostProcessPipeline,
    pub shadow: Arc<ShadowPipeline>,
}

impl Node for RenderPass {
    fn render(&self, ctx: &mut RenderContext<'_>) {
        let mut state = self.state.lock();

        state.update_buffers(
            ctx.device,
            ctx.queue,
            &self.forward,
            &self.shadow,
            ctx.mipmap,
        );

        for cam in state.camera_buffers.values() {
            if cam.target == RenderTarget::Window(ctx.window) {
                self.render_camera_target(&state, cam, ctx);
                return;
            }
        }

        // Some APIs don't play nicely when not submitting any work
        // for the surface, so we just clear the surface color.
        clear_pass(ctx);
    }
}

impl RenderPass {
    fn render_shadows(&self, state: &RenderState, ctx: &mut RenderContext<'_>) {
        for light in state.directional_lights.values() {
            self.render_directional_light_shadow(*light, state, ctx);
        }
    }

    fn render_directional_light_shadow(
        &self,
        light: DirectionalLight,
        state: &RenderState,
        ctx: &mut RenderContext<'_>,
    ) {
        let view = Mat4::look_to_rh(
            light.transform.translation,
            light.transform.rotation * -Vec3::Z,
            light.transform.rotation * Vec3::Y,
        );
        let proj = Mat4::orthographic_rh(-10.0, 10.0, -10.0, 10.0, 0.1, 1000.0);
        let view_proj = (OPENGL_TO_WGPU * proj * view).to_cols_array_2d();

        let buffer = ctx.device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: bytemuck::bytes_of(&view_proj),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let bind_groups = state
            .objects
            .keys()
            .map(|&id| {
                let transform = state.object_buffers.get(&id).unwrap();

                ctx.device.create_bind_group(&BindGroupDescriptor {
                    label: Some("shadow_bind_group"),
                    layout: &self.shadow.light_bind_group_layout,
                    entries: &[
                        BindGroupEntry {
                            binding: 0,
                            resource: buffer.as_entire_binding(),
                        },
                        BindGroupEntry {
                            binding: 1,
                            resource: transform.as_entire_binding(),
                        },
                    ],
                })
            })
            .collect::<Vec<_>>();

        let size = Extent3d {
            width: ctx.width,
            height: ctx.height,
            depth_or_array_layers: 1,
        };
        let shadow_map = ctx.device.create_texture(&TextureDescriptor {
            label: None,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Depth32Float,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let shadow_map_view = shadow_map.create_view(&TextureViewDescriptor::default());

        let mut render_pass = ctx.encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("shadow_pass"),
            color_attachments: &[],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: &shadow_map_view,
                depth_ops: Some(Operations {
                    load: LoadOp::Clear(1.0),
                    store: true,
                }),
                stencil_ops: None,
            }),
        });

        render_pass.set_pipeline(&self.shadow.pipeline);

        for (index, obj) in state.objects.values().enumerate() {
            let light_bg = &bind_groups[index];
            let mesh_data = state.meshes.get(&obj.mesh).unwrap();

            render_pass.set_bind_group(0, light_bg, &[]);
            render_pass.set_bind_group(1, &mesh_data.shadow_bind_group, &[]);

            render_pass.set_index_buffer(
                mesh_data.index_buffer.buffer.slice(..),
                mesh_data.index_buffer.format,
            );
            render_pass.draw_indexed(0..mesh_data.index_buffer.len, 0, 0..1);
        }
    }

    fn render_camera_target(
        &self,
        state: &RenderState,
        camera: &CameraBuffer,
        ctx: &mut RenderContext<'_>,
    ) {
        let _span = trace_span!("ForwardPass::render_camera_target").entered();

        self.render_shadows(state, ctx);

        let device = ctx.device;
        let pipeline = &self.forward;

        let bind_groups = state
            .objects
            .keys()
            .map(|&id| {
                let transform = state.object_buffers.get(&id).unwrap();

                device.create_bind_group(&BindGroupDescriptor {
                    label: Some("vs_bind_group"),
                    layout: &pipeline.vs_bind_group_layout,
                    entries: &[
                        BindGroupEntry {
                            binding: 0,
                            resource: camera.buffer.as_entire_binding(),
                        },
                        BindGroupEntry {
                            binding: 1,
                            resource: transform.as_entire_binding(),
                        },
                    ],
                })
            })
            .collect::<Vec<_>>();

        let light_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("light_bind_group"),
            layout: &pipeline.lights_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: state.directional_lights_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: state.point_lights_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: state.spot_lights_buffer.as_entire_binding(),
                },
            ],
        });

        let size = Extent3d {
            width: ctx.width,
            height: ctx.height,
            depth_or_array_layers: 1,
        };
        let render_target = device.create_texture(&TextureDescriptor {
            label: None,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba16Float,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let target_view = render_target.create_view(&TextureViewDescriptor::default());

        let mut render_pass = ctx.encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("render_pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &target_view,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color::BLACK),
                    store: true,
                },
            })],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: &ctx.surface.depth.view,
                depth_ops: Some(Operations {
                    load: LoadOp::Clear(1.0),
                    store: true,
                }),
                stencil_ops: None,
            }),
        });

        render_pass.set_pipeline(&pipeline.pipeline);

        for (index, obj) in state.objects.values().enumerate() {
            let vs_bind_group = &bind_groups[index];
            let mesh_data = state.meshes.get(&obj.mesh).unwrap();
            let mat_bg = state.materials.get(&obj.material).unwrap();

            render_pass.set_bind_group(0, vs_bind_group, &[]);
            render_pass.set_bind_group(1, &mesh_data.bind_group, &[]);
            render_pass.set_bind_group(2, mat_bg, &[]);
            render_pass.set_bind_group(3, &light_bind_group, &[]);

            render_pass.set_index_buffer(
                mesh_data.index_buffer.buffer.slice(..),
                mesh_data.index_buffer.format,
            );
            render_pass.draw_indexed(0..mesh_data.index_buffer.len, 0, 0..1);
        }

        drop(render_pass);

        self.post_process
            .render(ctx.encoder, &target_view, ctx.target, device, ctx.format);
    }
}

fn clear_pass(ctx: &mut RenderContext<'_>) {
    ctx.encoder.begin_render_pass(&RenderPassDescriptor {
        label: Some("clear_pass"),
        color_attachments: &[Some(RenderPassColorAttachment {
            view: ctx.target,
            resolve_target: None,
            ops: Operations {
                load: LoadOp::Clear(Color::BLACK),
                store: true,
            },
        })],
        depth_stencil_attachment: None,
    });
}
