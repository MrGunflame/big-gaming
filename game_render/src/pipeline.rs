use bevy_ecs::prelude::Entity;
use bevy_ecs::system::Resource;
use bevy_ecs::world::FromWorld;
use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{
    BindGroup, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
    BindingType, BlendState, Buffer, BufferBindingType, BufferUsages, ColorTargetState,
    ColorWrites, Device, Face, FragmentState, FrontFace, IndexFormat, MultisampleState, Operations,
    PipelineLayoutDescriptor, PolygonMode, PrimitiveState, PrimitiveTopology,
    RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor,
    ShaderModule, ShaderModuleDescriptor, ShaderSource, ShaderStages, TextureFormat, VertexState,
};

use crate::camera::{Projection, Transform, OPENGL_TO_WGPU};
use crate::graph::Node;
use crate::mesh::{Mesh, Vertex};
use crate::RenderDevice;

#[derive(Resource)]
pub struct MeshPipeline {
    pub bind_group_layout: BindGroupLayout,
    pub shader: ShaderModule,
    pub camera_buffer: Buffer,
}

impl FromWorld for MeshPipeline {
    fn from_world(world: &mut bevy_ecs::world::World) -> Self {
        world.resource_scope::<RenderDevice, _>(|_, device| Self::new(&device.0))
    }
}

impl MeshPipeline {
    pub fn new(device: &Device) -> Self {
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("camera_bind_group_layout"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("mesh_shader"),
            source: ShaderSource::Wgsl(include_str!("mesh.wgsl").into()),
        });

        let camera_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("camera_buffer"),
            contents: bytemuck::cast_slice(&[CameraUniform::default()]),
            usage: wgpu::BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        Self {
            bind_group_layout,
            shader,
            camera_buffer,
        }
    }
}

#[derive(Debug, Resource)]
pub struct MaterialPipeline {
    pipeline: RenderPipeline,
}

impl FromWorld for MaterialPipeline {
    fn from_world(world: &mut bevy_ecs::world::World) -> Self {
        world.resource_scope::<RenderDevice, _>(|world, device| {
            world.resource_scope::<MeshPipeline, _>(|_, mesh_pipeline| {
                Self::new(&device.0, &mesh_pipeline)
            })
        })
    }
}

impl MaterialPipeline {
    pub fn new(device: &Device, mesh_pipeline: &MeshPipeline) -> Self {
        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("render_pipeline_layout"),
            bind_group_layouts: &[&mesh_pipeline.bind_group_layout],
            push_constant_ranges: &[],
        });

        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("material_shader"),
            source: ShaderSource::Wgsl(include_str!("material.wgsl").into()),
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("mesh_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &mesh_pipeline.shader,
                entry_point: "vs_main",
                buffers: &[Vertex::layout()],
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(ColorTargetState {
                    format: TextureFormat::Bgra8UnormSrgb,
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

        Self { pipeline }
    }
}

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
pub struct CameraUniform {
    view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    pub fn new(transform: Transform, projection: Projection) -> Self {
        let view = Mat4::look_to_rh(
            transform.translation,
            (transform.rotation * -Vec3::Z) - transform.translation,
            Vec3::Y,
        );

        let proj = Mat4::perspective_rh(
            projection.fov,
            projection.aspect_ratio,
            projection.near,
            projection.far,
        );

        Self {
            view_proj: (super::camera::OPENGL_TO_WGPU * proj * view).to_cols_array_2d(),
        }
    }
}

impl From<Mat4> for CameraUniform {
    fn from(value: Mat4) -> Self {
        Self {
            view_proj: value.to_cols_array_2d(),
        }
    }
}

impl Default for CameraUniform {
    fn default() -> Self {
        Self::new(Transform::default(), Projection::default())
    }
}

#[derive(Debug)]
struct RenderNode {
    vertices: Buffer,
    indices: Buffer,
    num_vertices: u32,
    bind_group: BindGroup,
}

#[derive(Debug, Default)]
pub struct MainPass {
    nodes: Vec<RenderNode>,
}

impl Node for MainPass {
    fn update(&mut self, world: &mut bevy_ecs::world::World) {
        world.resource_scope::<RenderDevice, _>(|world, device| {
            world.resource_scope::<MeshPipeline, _>(|world, pipeline| {
                let mut query = world.query::<(Entity, &Mesh)>();

                self.nodes.clear();

                for (entity, mesh) in query.iter(&world) {
                    let vertices = device.0.create_buffer_init(&BufferInitDescriptor {
                        label: Some("mesh_vertex_buffer"),
                        contents: bytemuck::cast_slice(&mesh.vertices()),
                        usage: BufferUsages::VERTEX,
                    });

                    let indices = mesh.indicies().unwrap();
                    let num_vertices = indices.len() as u32;

                    let indices = device.0.create_buffer_init(&BufferInitDescriptor {
                        label: Some("mesh_index_buffer"),
                        contents: bytemuck::cast_slice(indices.as_u32()),
                        usage: BufferUsages::INDEX,
                    });

                    let bind_group = device.0.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("mesh_bind_group"),
                        layout: &pipeline.bind_group_layout,
                        entries: &[BindGroupEntry {
                            binding: 0,
                            resource: pipeline.camera_buffer.as_entire_binding(),
                        }],
                    });

                    self.nodes.push(RenderNode {
                        vertices,
                        indices,
                        num_vertices,
                        bind_group,
                    });
                }
            });
        });
    }

    fn render(&self, world: &bevy_ecs::world::World, ctx: &mut crate::graph::RenderContext<'_>) {
        let pipeline = world.resource::<MaterialPipeline>();

        let mut render_pass = ctx.encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("main_pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &ctx.view,
                resolve_target: None,
                ops: Operations {
                    load: wgpu::LoadOp::Load,
                    store: true,
                },
            })],
            depth_stencil_attachment: None,
        });

        render_pass.set_pipeline(&pipeline.pipeline);

        for node in &self.nodes {
            render_pass.set_bind_group(0, &node.bind_group, &[]);
            render_pass.set_vertex_buffer(0, node.vertices.slice(..));
            render_pass.set_index_buffer(node.indices.slice(..), IndexFormat::Uint32);

            render_pass.draw_indexed(0..node.num_vertices, 0, 0..1);
        }
    }
}
