use bevy_ecs::prelude::{Component, Entity, Res};
use bevy_ecs::query::{Added, Changed, With};
use bevy_ecs::system::{Commands, Query, Resource};
use bevy_ecs::world::FromWorld;
use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{
    AddressMode, BindGroup, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, BlendState, Buffer, BufferBindingType,
    BufferUsages, ColorTargetState, ColorWrites, Device, Extent3d, Face, FilterMode, FragmentState,
    FrontFace, ImageCopyTexture, ImageDataLayout, IndexFormat, MultisampleState, Operations,
    Origin3d, PipelineLayoutDescriptor, PolygonMode, PrimitiveState, PrimitiveTopology,
    RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor,
    Sampler, SamplerBindingType, SamplerDescriptor, ShaderModule, ShaderModuleDescriptor,
    ShaderSource, ShaderStages, StencilState, Texture, TextureAspect, TextureDescriptor,
    TextureDimension, TextureFormat, TextureSampleType, TextureUsages, TextureView,
    TextureViewDescriptor, TextureViewDimension, VertexState,
};

use crate::camera::{Projection, Transform};
use crate::graph::Node;
use crate::material::Material;
use crate::mesh::{Mesh, Vertex};
use crate::{RenderDevice, RenderQueue};

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
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
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
    bind_group_layout: BindGroupLayout,
    sampler: Sampler,
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
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("material_bind_group_layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("render_pipeline_layout"),
            bind_group_layouts: &[&mesh_pipeline.bind_group_layout, &bind_group_layout],
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

        let sampler = device.create_sampler(&SamplerDescriptor {
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Nearest,
            mipmap_filter: FilterMode::Nearest,
            ..Default::default()
        });

        Self {
            pipeline,
            bind_group_layout,
            sampler,
        }
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
    bind_groups: Vec<BindGroup>,
}

#[derive(Debug, Default)]
pub struct MainPass {
    nodes: Vec<RenderNode>,
}

impl Node for MainPass {
    fn update(&mut self, world: &mut bevy_ecs::world::World) {
        world.resource_scope::<RenderDevice, _>(|world, device| {
            world.resource_scope::<RenderQueue, _>(|world, queue| {
                world.resource_scope::<MeshPipeline, _>(|world, pipeline| {
                    world.resource_scope::<MaterialPipeline, _>(|world, mat_pl| {
                        let mut query =
                            world.query::<(Entity, &Mesh, &Material, &TransformationMatrix)>();

                        self.nodes.clear();

                        for (entity, mesh, material, mat) in query.iter(&world) {
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

                            let bind_group =
                                device.0.create_bind_group(&wgpu::BindGroupDescriptor {
                                    label: Some("mesh_bind_group"),
                                    layout: &pipeline.bind_group_layout,
                                    entries: &[
                                        BindGroupEntry {
                                            binding: 0,
                                            resource: pipeline.camera_buffer.as_entire_binding(),
                                        },
                                        BindGroupEntry {
                                            binding: 1,
                                            resource: mat.buffer.as_entire_binding(),
                                        },
                                    ],
                                });

                            let base_color = device.0.create_buffer_init(&BufferInitDescriptor {
                                label: Some("base_color"),
                                contents: bytemuck::cast_slice(&[material.color]),
                                usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                            });

                            let base_texture = device.0.create_texture(&TextureDescriptor {
                                size: wgpu::Extent3d {
                                    width: material.color_texture.width(),
                                    height: material.color_texture.height(),
                                    depth_or_array_layers: 1,
                                },
                                mip_level_count: 1,
                                sample_count: 1,
                                dimension: TextureDimension::D2,
                                format: TextureFormat::Rgba8UnormSrgb,
                                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
                                label: Some("base_color_texture"),
                                view_formats: &[],
                            });

                            queue.0.write_texture(
                                ImageCopyTexture {
                                    texture: &base_texture,
                                    mip_level: 0,
                                    origin: Origin3d::ZERO,
                                    aspect: TextureAspect::All,
                                },
                                &material.color_texture,
                                ImageDataLayout {
                                    offset: 0,
                                    bytes_per_row: Some(4 * material.color_texture.width()),
                                    rows_per_image: Some(material.color_texture.height()),
                                },
                                Extent3d {
                                    width: material.color_texture.width(),
                                    height: material.color_texture.height(),
                                    depth_or_array_layers: 1,
                                },
                            );

                            let texture_view =
                                base_texture.create_view(&TextureViewDescriptor::default());

                            let bind_group_mat =
                                device.0.create_bind_group(&wgpu::BindGroupDescriptor {
                                    label: Some("material_bind_group"),
                                    layout: &mat_pl.bind_group_layout,
                                    entries: &[
                                        BindGroupEntry {
                                            binding: 0,
                                            resource: base_color.as_entire_binding(),
                                        },
                                        BindGroupEntry {
                                            binding: 1,
                                            resource: BindingResource::TextureView(&texture_view),
                                        },
                                        BindGroupEntry {
                                            binding: 2,
                                            resource: BindingResource::Sampler(&mat_pl.sampler),
                                        },
                                    ],
                                });

                            self.nodes.push(RenderNode {
                                vertices,
                                indices,
                                num_vertices,
                                bind_groups: vec![bind_group, bind_group_mat],
                            });
                        }
                    });
                });
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
            for (group, bind_group) in node.bind_groups.iter().enumerate() {
                render_pass.set_bind_group(group as u32, bind_group, &[]);
            }

            render_pass.set_vertex_buffer(0, node.vertices.slice(..));
            render_pass.set_index_buffer(node.indices.slice(..), IndexFormat::Uint32);

            render_pass.draw_indexed(0..node.num_vertices, 0, 0..1);
        }
    }
}

#[derive(Debug, Component)]
pub struct TransformationMatrix {
    pub mat: Mat4,
    pub buffer: Buffer,
}

pub fn create_transformatio_matrix(
    device: Res<RenderDevice>,
    mut commands: Commands,
    meshes: Query<(Entity, &Transform), (With<Mesh>, Added<Transform>)>,
) {
    for (entity, transform) in &meshes {
        let mat = transform.compute_matrix();

        let buffer = device.0.create_buffer_init(&BufferInitDescriptor {
            label: Some("transform_matrix_buffer"),
            contents: bytemuck::cast_slice(&[mat]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        commands
            .entity(entity)
            .insert(TransformationMatrix { mat, buffer });
    }
}

pub fn update_transformation_matrix(
    device: Res<RenderDevice>,
    mut meshes: Query<(&Transform, &mut TransformationMatrix), Changed<Transform>>,
) {
    for (transform, mut mat) in &mut meshes {
        mat.mat = transform.compute_matrix();

        let buffer = device.0.create_buffer_init(&BufferInitDescriptor {
            label: Some("transform_matrix_buffer"),
            contents: bytemuck::cast_slice(&[mat.mat]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        mat.buffer = buffer;
    }
}