use std::collections::HashMap;
use std::sync::Arc;

use game_tracing::trace_span;
use glam::UVec2;
use parking_lot::Mutex;
use wgpu::{
    BindGroupDescriptor, BindGroupEntry, Color, Device, Extent3d, LoadOp, Operations,
    RenderPassColorAttachment, RenderPassDepthStencilAttachment, RenderPassDescriptor,
    ShaderStages, StoreOp, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
    TextureViewDescriptor,
};

use crate::camera::{Camera, CameraUniform, RenderTarget};
use crate::depth_stencil::DepthData;
use crate::forward::ForwardPipeline;
use crate::graph::{Node, RenderContext, SlotLabel};
use crate::options::MainPassOptionsEncoded;
use crate::state::RenderState;

pub(super) struct ForwardPass {
    pub state: Arc<Mutex<HashMap<RenderTarget, RenderState>>>,
    pub forward: Arc<ForwardPipeline>,
    pub depth_stencils: Mutex<HashMap<RenderTarget, DepthData>>,
    pub dst: SlotLabel,
}

impl ForwardPass {
    pub(super) fn new(
        state: Arc<Mutex<HashMap<RenderTarget, RenderState>>>,
        forward: Arc<ForwardPipeline>,
        dst: SlotLabel,
    ) -> Self {
        Self {
            state,
            forward,
            depth_stencils: Mutex::default(),
            dst,
        }
    }
}

impl Node for ForwardPass {
    fn render(&self, ctx: &mut RenderContext<'_, '_>) {
        let mut state = self.state.lock();
        if let Some(state) = state.get_mut(&ctx.render_target) {
            state.update_buffers(ctx.device, ctx.queue, &self.forward, ctx.mipmap);

            for cam in state.cameras.values() {
                if cam.target == ctx.render_target {
                    self.update_depth_stencil(ctx.render_target, ctx.size, ctx.device);

                    self.render_camera_target(state, cam, ctx);
                    return;
                }
            }
        }

        // Some APIs don't play nicely when not submitting any work
        // for the surface, so we just clear the surface color.
        clear_pass(ctx, self.dst);
    }
}

impl ForwardPass {
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
        camera: &Camera,
        ctx: &mut RenderContext<'_, '_>,
    ) {
        let _span = trace_span!("ForwardPass::render_camera_target").entered();

        let device = ctx.device;
        let pipeline = &self.forward;
        let depth_stencils = self.depth_stencils.lock();

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
        push_constants[80..84].copy_from_slice(bytemuck::bytes_of(&MainPassOptionsEncoded::new(
            &state.options,
        )));

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
        ctx.write(self.dst, render_target).unwrap();
    }
}

fn clear_pass(ctx: &mut RenderContext<'_, '_>, dst: SlotLabel) {
    let texture = ctx.device.create_texture(&TextureDescriptor {
        label: None,
        size: Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Rgba16Float,
        usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = texture.create_view(&TextureViewDescriptor::default());

    ctx.encoder.begin_render_pass(&RenderPassDescriptor {
        label: Some("clear_pass"),
        color_attachments: &[Some(RenderPassColorAttachment {
            view: &view,
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

    ctx.write(dst, texture).unwrap();
}
