use std::collections::HashMap;
use std::sync::Arc;

use game_tracing::trace_span;
use glam::UVec2;
use parking_lot::Mutex;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, Buffer, BufferUsages, Color, Device, Extent3d,
    LoadOp, Operations, RenderPassColorAttachment, RenderPassDepthStencilAttachment,
    RenderPassDescriptor, ShaderStages, StoreOp, TextureDescriptor, TextureDimension,
    TextureFormat, TextureUsages, TextureViewDescriptor,
};

use crate::buffer::{DynamicBuffer, IndexBuffer};
use crate::camera::{CameraBuffer, CameraUniform, RenderTarget};
use crate::depth_stencil::DepthData;
use crate::entities::{CameraId, ObjectId};
use crate::forward::ForwardPipeline;
use crate::graph::{Node, RenderContext};
use crate::light::pipeline::{DirectionalLightUniform, PointLightUniform, SpotLightUniform};
use crate::options::{MainPassOptions, MainPassOptionsEncoded};
use crate::post_process::PostProcessPipeline;
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
    pub depth_stencils: Mutex<HashMap<RenderTarget, DepthData>>,
    pub options: Arc<Mutex<MainPassOptions>>,
}

impl Node for RenderPass {
    fn render(&self, ctx: &mut RenderContext<'_>) {
        let mut state = self.state.lock();

        state.update_buffers(ctx.device, ctx.queue, &self.forward, ctx.mipmap);

        for cam in state.camera_buffers.values() {
            if cam.target == ctx.render_target {
                self.update_depth_stencil(ctx.render_target, ctx.size, ctx.device);

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
    fn update_depth_stencil(&self, target: RenderTarget, size: UVec2, device: &Device) {
        let mut depth_stencils = self.depth_stencils.lock();

        if let Some(data) = depth_stencils.get(&target) {
            // Texture size unchanged.
            if data.texture.width() == size.x && data.texture.height() == size.y {
                return;
            }
        }

        depth_stencils.insert(target, DepthData::new(device, size));
    }

    fn render_camera_target(
        &self,
        state: &RenderState,
        camera: &CameraBuffer,
        ctx: &mut RenderContext<'_>,
    ) {
        let _span = trace_span!("ForwardPass::render_camera_target").entered();

        let device = ctx.device;
        let pipeline = &self.forward;
        let depth_stencils = self.depth_stencils.lock();
        let options = self.options.lock();

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

        let depth_stencil = depth_stencils.get(&ctx.render_target).unwrap();

        let size = Extent3d {
            width: ctx.size.x,
            height: ctx.size.y,
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
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: &depth_stencil.view,
                depth_ops: Some(Operations {
                    load: LoadOp::Clear(1.0),
                    store: StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        let mut push_constants = [0; 84];
        push_constants[0..80].copy_from_slice(bytemuck::bytes_of(&CameraUniform::new(
            camera.transform,
            camera.projection,
        )));
        push_constants[80..84]
            .copy_from_slice(bytemuck::bytes_of(&MainPassOptionsEncoded::new(&options)));

        render_pass.set_pipeline(&pipeline.pipeline);
        render_pass.set_push_constants(
            ShaderStages::VERTEX | ShaderStages::FRAGMENT,
            0,
            &push_constants,
        );

        for (id, obj) in state.objects.iter() {
            let vs_bind_group = state.object_buffers.get(id).unwrap();
            let (mesh_bg, idx_buf) = state.meshes.get(&obj.mesh).unwrap();
            let mat_bg = state.materials.get(&obj.material).unwrap();

            render_pass.set_bind_group(0, vs_bind_group, &[]);
            render_pass.set_bind_group(1, mesh_bg, &[]);
            render_pass.set_bind_group(2, mat_bg, &[]);
            render_pass.set_bind_group(3, &light_bind_group, &[]);

            render_pass.set_index_buffer(idx_buf.buffer.slice(..), idx_buf.format);
            render_pass.draw_indexed(0..idx_buf.len, 0, 0..1);
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
                store: StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        occlusion_query_set: None,
        timestamp_writes: None,
    });
}
