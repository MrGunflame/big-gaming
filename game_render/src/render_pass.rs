use std::collections::HashMap;

use bevy_ecs::prelude::Entity;
use bevy_ecs::system::Resource;
use bevy_ecs::world::{FromWorld, World};
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, Buffer, BufferUsages, Color, LoadOp,
    Operations, RenderPassColorAttachment, RenderPassDepthStencilAttachment, RenderPassDescriptor,
};

use crate::buffer::{DynamicBuffer, IndexBuffer};
use crate::camera::{CameraBuffer, Cameras};
use crate::depth_stencil::DepthTextures;
use crate::forward::ForwardPipeline;
use crate::graph::{Node, RenderContext};
use crate::light::pipeline::{DirectionalLightUniform, PointLightUniform, SpotLightUniform};
use crate::RenderDevice;

#[derive(Resource)]
pub struct RenderNodes {
    pub entities: HashMap<Entity, RenderNode>,
    pub directional_lights: Buffer,
    pub point_lights: Buffer,
    pub spot_lights: Buffer,
}

impl FromWorld for RenderNodes {
    fn from_world(world: &mut World) -> Self {
        let device = world.resource::<RenderDevice>();

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
            entities: HashMap::default(),
            directional_lights,
            point_lights,
            spot_lights,
        }
    }
}

#[derive(Debug, Default)]
pub struct RenderNode {
    pub vertices: Option<Buffer>,
    pub indices: Option<IndexBuffer>,
    pub transform: Option<Buffer>,
    pub mesh_bind_group: Option<BindGroup>,
    pub material_bind_group: Option<BindGroup>,
}

impl RenderNode {
    fn is_ready(&self) -> bool {
        self.vertices.is_some()
            && self.indices.is_some()
            && self.transform.is_some()
            && self.mesh_bind_group.is_some()
            && self.material_bind_group.is_some()
    }
}

pub struct RenderPass;

impl Node for RenderPass {
    fn update(&mut self, world: &mut World) {}

    fn render(&self, world: &World, ctx: &mut RenderContext<'_>) {
        let cameras = world.resource::<Cameras>();

        if let Some(entity) = cameras.window_targets.get(&ctx.window) {
            if let Some(camera) = cameras.cameras.get(&entity) {
                self.render_camera_target(camera, world, ctx);
            }
        }
    }
}

impl RenderPass {
    fn render_camera_target(
        &self,
        camera: &CameraBuffer,
        world: &World,
        ctx: &mut RenderContext<'_>,
    ) {
        let device = world.resource::<RenderDevice>();
        let pipeline = world.resource::<ForwardPipeline>();
        let nodes = world.resource::<RenderNodes>();
        let depth_textures = world.resource::<DepthTextures>();

        let depth_texture = depth_textures.windows.get(&ctx.window).unwrap();

        let bind_groups = nodes
            .entities
            .values()
            .filter(|node| node.is_ready())
            .map(|node| {
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
                            resource: node.transform.as_ref().unwrap().as_entire_binding(),
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
                    resource: nodes.directional_lights.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: nodes.point_lights.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: nodes.spot_lights.as_entire_binding(),
                },
            ],
        });

        let mut render_pass = ctx.encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("render_pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &ctx.target,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color::BLACK),
                    store: true,
                },
            })],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: &depth_texture.view,
                depth_ops: Some(Operations {
                    load: LoadOp::Clear(1.0),
                    store: true,
                }),
                stencil_ops: None,
            }),
        });

        render_pass.set_pipeline(&pipeline.pipeline);

        for (index, node) in nodes.entities.values().enumerate() {
            if !node.is_ready() {
                continue;
            }

            let vs_bind_group = &bind_groups[index];

            render_pass.set_bind_group(0, &vs_bind_group, &[]);
            render_pass.set_bind_group(1, node.mesh_bind_group.as_ref().unwrap(), &[]);
            render_pass.set_bind_group(2, node.material_bind_group.as_ref().unwrap(), &[]);
            render_pass.set_bind_group(3, &light_bind_group, &[]);

            render_pass.set_index_buffer(
                node.indices.as_ref().unwrap().buffer.slice(..),
                node.indices.as_ref().unwrap().format,
            );
            render_pass.draw_indexed(0..node.indices.as_ref().unwrap().len, 0, 0..1);
        }
    }
}
