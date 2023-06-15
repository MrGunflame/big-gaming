use std::collections::HashMap;
use std::sync::Arc;

use bevy_ecs::prelude::{Component, Entity, EventReader, Res};
use bevy_ecs::query::{Added, Changed, With};
use bevy_ecs::system::{Commands, Query, ResMut, Resource};
use bevy_ecs::world::FromWorld;
use bytemuck::{Pod, Zeroable};
use game_common::components::transform::Transform;
use game_window::events::{WindowCreated, WindowDestroyed, WindowResized};
use game_window::WindowState;
use glam::{Mat3, Mat4, UVec2, Vec3, Vec4};
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{
    AddressMode, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType, BlendComponent,
    BlendFactor, BlendOperation, BlendState, Buffer, BufferAddress, BufferBindingType,
    BufferDescriptor, BufferUsages, Color, ColorTargetState, ColorWrites, CompareFunction,
    DepthBiasState, DepthStencilState, Device, Extent3d, Face, FilterMode, FragmentState,
    FrontFace, ImageCopyBuffer, ImageCopyTexture, IndexFormat, LoadOp, MultisampleState,
    Operations, PipelineLayoutDescriptor, PolygonMode, PrimitiveState, PrimitiveTopology,
    RenderPassColorAttachment, RenderPassDepthStencilAttachment, RenderPassDescriptor,
    RenderPipeline, RenderPipelineDescriptor, Sampler, SamplerBindingType, SamplerDescriptor,
    ShaderModule, ShaderModuleDescriptor, ShaderSource, ShaderStages, StencilState, Texture,
    TextureDescriptor, TextureDimension, TextureFormat, TextureSampleType, TextureUsages,
    TextureView, TextureViewDescriptor, TextureViewDimension, VertexAttribute, VertexBufferLayout,
    VertexFormat, VertexState, VertexStepMode,
};

use crate::camera::{Projection, OPENGL_TO_WGPU};
use crate::depth_stencil::{create_depth_texture, DEPTH_TEXTURE_FORMAT};
use crate::graph::Node;
use crate::mesh::{Mesh, Vertex};
use crate::pbr::RenderMaterialAssets;
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
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
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
    pub pipeline: RenderPipeline,
    pub bind_group_layout: BindGroupLayout,
    pub sampler: Sampler,
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
                // NORMAL TEXTURE
                BindGroupLayoutEntry {
                    binding: 3,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 4,
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

        let depth_stencil = DepthStencilState {
            format: DEPTH_TEXTURE_FORMAT,
            depth_write_enabled: true,
            depth_compare: CompareFunction::Less,
            stencil: StencilState::default(),
            bias: DepthBiasState::default(),
        };

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
                targets: GBuffer::targets(),
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
            depth_stencil: Some(depth_stencil),
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
    // We only need `[f32; 3]`, but one word for alignment.
    view_position: [f32; 4],
    view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    pub fn new(transform: Transform, projection: Projection) -> Self {
        let view = Mat4::look_to_rh(
            transform.translation,
            transform.rotation * -Vec3::Z,
            transform.rotation * Vec3::Y,
        );

        let proj = Mat4::perspective_rh(
            projection.fov,
            projection.aspect_ratio,
            projection.near,
            projection.far,
        );

        Self {
            view_position: [
                transform.translation.x,
                transform.translation.y,
                transform.translation.z,
                0.0,
            ],
            view_proj: (OPENGL_TO_WGPU * proj * view).to_cols_array_2d(),
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
    vertices: Arc<Buffer>,
    indices: Arc<Buffer>,
    num_vertices: u32,
    bind_groups: Vec<BindGroup>,
}

#[derive(Debug, Default)]
pub struct MainPass {
    nodes: Vec<RenderNode>,
}

impl Node for MainPass {
    fn update(&mut self, world: &mut bevy_ecs::world::World) {
        // world.resource_scope::<RenderDevice, _>(|world, device| {
        //     world.resource_scope::<MeshPipeline, _>(|world, pipeline| {
        //         world.resource_scope::<MaterialPipeline, _>(|world, mat_pl| {
        //             let mut nodes = world.resource_mut::<RenderMaterialAssets>();

        //             for node in &mut nodes.entities {}

        //             let mut query =
        //                 world.query::<(&ComputedMesh, &ComputedMaterial, &TransformationMatrix)>();

        //             self.nodes.clear();

        //             for (mesh, material, mat) in query.iter(&world) {
        //                 let Some(vertices) = &mesh.vertices else {
        //                         continue;
        //                     };
        //                 let Some(indices) = &mesh.indicies else {
        //                         continue;
        //                     };
        //                 let num_vertices = mesh.num_vertices;

        //                 let bind_group = device.0.create_bind_group(&wgpu::BindGroupDescriptor {
        //                     label: Some("mesh_bind_group"),
        //                     layout: &pipeline.bind_group_layout,
        //                     entries: &[
        //                         BindGroupEntry {
        //                             binding: 0,
        //                             resource: pipeline.camera_buffer.as_entire_binding(),
        //                         },
        //                         BindGroupEntry {
        //                             binding: 1,
        //                             resource: mat.buffer.as_entire_binding(),
        //                         },
        //                     ],
        //                 });

        //                 let Some(base_color) = material.base_color.as_ref() else {
        //                         continue;
        //                     };

        //                 let Some(texture_view) = material
        //                         .base_color_texture
        //                         .as_ref()
        //                         .map(|t| t.create_view(&TextureViewDescriptor::default())) else {
        //                             continue;
        //                         };

        //                 let bind_group_mat =
        //                     device.0.create_bind_group(&wgpu::BindGroupDescriptor {
        //                         label: Some("material_bind_group"),
        //                         layout: &mat_pl.bind_group_layout,
        //                         entries: &[
        //                             BindGroupEntry {
        //                                 binding: 0,
        //                                 resource: base_color.as_entire_binding(),
        //                             },
        //                             BindGroupEntry {
        //                                 binding: 1,
        //                                 resource: BindingResource::TextureView(&texture_view),
        //                             },
        //                             BindGroupEntry {
        //                                 binding: 2,
        //                                 resource: BindingResource::Sampler(&mat_pl.sampler),
        //                             },
        //                         ],
        //                     });

        //                 self.nodes.push(RenderNode {
        //                     vertices: vertices.clone(),
        //                     indices: indices.clone(),
        //                     num_vertices,
        //                     bind_groups: vec![bind_group, bind_group_mat],
        //                 });
        //             }
        //         });
        //     });
        // });
    }

    fn render(&self, world: &bevy_ecs::world::World, ctx: &mut crate::graph::RenderContext<'_>) {
        let pipeline = world.resource::<MaterialPipeline>();
        let windows = world.resource::<RenderWindows>();
        let nodes = world.resource::<RenderMaterialAssets>();

        let Some(window) = windows.windows.get(&ctx.window) else {
            return;
        };

        // FIXME: We should avoid this check by making sure any resizes
        // happen before a render.
        if window.depth_texture.width() != ctx.width || window.depth_texture.height() != ctx.height
        {
            tracing::error!(
                "depth texture is ({}, {}) but render target is ({}, {})",
                window.depth_texture.width(),
                window.depth_texture.height(),
                ctx.width,
                ctx.height,
            );

            return;
        }

        // Geometry pass

        let position_view = window
            .g_buffer
            .position
            .create_view(&TextureViewDescriptor::default());
        let normal_view = window
            .g_buffer
            .normal
            .create_view(&TextureViewDescriptor::default());

        let albdeo_view = window
            .g_buffer
            .albedo
            .create_view(&TextureViewDescriptor::default());

        let mut render_pass = ctx.encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("main_pass"),
            color_attachments: &[
                Some(RenderPassColorAttachment {
                    view: &position_view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::BLACK),
                        store: true,
                    },
                }),
                Some(RenderPassColorAttachment {
                    view: &normal_view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::BLACK),
                        store: true,
                    },
                }),
                Some(RenderPassColorAttachment {
                    view: &albdeo_view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::BLACK),
                        store: true,
                    },
                }),
            ],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: &window.depth_texture_view,
                depth_ops: Some(Operations {
                    load: LoadOp::Clear(1.0),
                    store: true,
                }),
                stencil_ops: None,
            }),
        });

        render_pass.set_pipeline(&pipeline.pipeline);

        for node in &nodes.entities {
            for (group, bind_group) in node.bind_groups.iter().enumerate() {
                render_pass.set_bind_group(group as u32, bind_group, &[]);
            }

            render_pass.set_vertex_buffer(0, node.vertices.slice(..));
            render_pass.set_index_buffer(node.indices.slice(..), IndexFormat::Uint32);

            render_pass.draw_indexed(0..node.num_vertices, 0, 0..1);
        }

        drop(render_pass);

        // Final pass
        {
            let device = world.resource::<RenderDevice>();
            let data = world.resource::<LightingPipeline>();
            let mesh_pipeline = world.resource::<MeshPipeline>();

            let bind_group = device.0.create_bind_group(&BindGroupDescriptor {
                label: Some("final_pass_bind_group"),
                layout: &data.bind_group_layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::TextureView(&position_view),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::Sampler(&data.sampler),
                    },
                    BindGroupEntry {
                        binding: 2,
                        resource: BindingResource::TextureView(&normal_view),
                    },
                    BindGroupEntry {
                        binding: 3,
                        resource: BindingResource::TextureView(&albdeo_view),
                    },
                    BindGroupEntry {
                        binding: 4,
                        resource: mesh_pipeline.camera_buffer.as_entire_binding(),
                    },
                ],
            });

            let mut render_pass = ctx.encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("render_pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &ctx.view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::BLACK),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            render_pass.set_pipeline(&data.pipeline);

            for node in &nodes.lights {
                render_pass.set_bind_group(0, &bind_group, &[]);
                render_pass.set_bind_group(1, &node.bind_group, &[]);

                render_pass.set_vertex_buffer(0, data.vertices.slice(..));
                render_pass.set_index_buffer(data.indices.slice(..), IndexFormat::Uint16);

                render_pass.draw_indexed(0..LightingPipeline::NUM_VERTICES, 0, 0..1);
            }
        }
    }
}

#[derive(Debug, Resource)]
pub struct LightingPipeline {
    pipeline: RenderPipeline,
    vertices: Buffer,
    indices: Buffer,
    bind_group_layout: BindGroupLayout,
    sampler: Sampler,
    pub light_bind_group_layout: BindGroupLayout,
}

impl LightingPipeline {
    const INDICES: &[u16] = &[0, 1, 2, 2, 3, 0];
    const NUM_VERTICES: u32 = Self::INDICES.len() as u32;

    fn new(device: &Device) -> Self {
        const VERTICES: &[Vertex] = &[
            Vertex {
                position: [-1.0, 1.0],
                uv: [0.0, 0.0],
            },
            Vertex {
                position: [-1.0, -1.0],
                uv: [0.0, 1.0],
            },
            Vertex {
                position: [1.0, -1.0],
                uv: [1.0, 1.0],
            },
            Vertex {
                position: [1.0, 1.0],
                uv: [1.0, 0.0],
            },
        ];

        #[derive(Copy, Clone, Debug, Zeroable, Pod)]
        #[repr(C)]
        struct Vertex {
            position: [f32; 2],
            uv: [f32; 2],
        }

        impl Vertex {
            fn layout<'a>() -> VertexBufferLayout<'a> {
                VertexBufferLayout {
                    array_stride: std::mem::size_of::<Self>() as BufferAddress,
                    step_mode: VertexStepMode::Vertex,
                    attributes: &[
                        VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: VertexFormat::Float32x2,
                        },
                        VertexAttribute {
                            offset: std::mem::size_of::<[f32; 2]>() as BufferAddress,
                            shader_location: 1,
                            format: VertexFormat::Float32x2,
                        },
                    ],
                }
            }
        }

        let vertices = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("lighting_pass_vertex_buffer"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: BufferUsages::VERTEX,
        });

        let indices = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("lighting_pass_index_buffer"),
            contents: bytemuck::cast_slice(Self::INDICES),
            usage: BufferUsages::INDEX,
        });

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("lighting_pass_bind_group_layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 3,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 4,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let light_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("lighting_pass_bind_group_layout"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("lighting_pass_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout, &light_bind_group_layout],
            push_constant_ranges: &[],
        });

        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("lighting_pass_shader"),
            source: ShaderSource::Wgsl(include_str!("lighting_pass.wgsl").into()),
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("lighting_pass_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::layout()],
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(ColorTargetState {
                    format: TextureFormat::Bgra8UnormSrgb,
                    blend: Some(BlendState {
                        color: BlendComponent {
                            src_factor: BlendFactor::One,
                            dst_factor: BlendFactor::One,
                            operation: BlendOperation::Add,
                        },
                        alpha: BlendComponent::OVER,
                    }),
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
            vertices,
            indices,
            bind_group_layout,
            sampler,
            light_bind_group_layout,
        }
    }
}

impl FromWorld for LightingPipeline {
    fn from_world(world: &mut bevy_ecs::world::World) -> Self {
        world.resource_scope::<RenderDevice, _>(|world, device| Self::new(&device.0))
    }
}

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
pub struct TransformUniform {
    transform: [[f32; 4]; 4],
    // rotation matrix for normals/tangents
    // Note that we can't use the transform matrix for non-uniform
    // scaling values.
    normal: [[f32; 4]; 3],
}

impl From<Transform> for TransformUniform {
    fn from(value: Transform) -> Self {
        let normal = Mat3::from_quat(value.rotation);
        let normal_x = Vec4::new(normal.x_axis.x, normal.x_axis.y, normal.x_axis.z, 0.0);
        let normal_y = Vec4::new(normal.y_axis.x, normal.y_axis.y, normal.y_axis.z, 0.0);
        let normal_z = Vec4::new(normal.z_axis.x, normal.z_axis.y, normal.z_axis.z, 0.0);

        Self {
            transform: Mat4::from_scale_rotation_translation(
                value.scale,
                value.rotation,
                value.translation,
            )
            .to_cols_array_2d(),
            normal: [
                normal_x.to_array(),
                normal_y.to_array(),
                normal_z.to_array(),
            ],
        }
    }
}

#[derive(Debug, Default, Resource)]
pub struct RenderWindows {
    windows: HashMap<Entity, WindowData>,
}

#[derive(Debug)]
pub struct WindowData {
    depth_texture: Texture,
    depth_texture_view: TextureView,
    depth_sampler: Sampler,
    g_buffer: GBuffer,
}

pub fn create_render_windows(
    device: Res<RenderDevice>,
    mut windows: ResMut<RenderWindows>,
    mut events: EventReader<WindowCreated>,
    window_query: Query<&WindowState>,
) {
    for event in events.iter() {
        let window = window_query.get(event.window).unwrap();
        let size = window.inner_size();

        let (depth_texture, depth_texture_view, depth_sampler) =
            create_depth_texture(&device.0, UVec2::new(size.width, size.height));

        windows.windows.insert(
            event.window,
            WindowData {
                depth_texture,
                depth_texture_view,
                depth_sampler,
                g_buffer: GBuffer::new(&device.0, size.width, size.height),
            },
        );
    }
}

pub fn destroy_render_windows(
    mut windows: ResMut<RenderWindows>,
    mut events: EventReader<WindowDestroyed>,
) {
    for event in events.iter() {
        windows.windows.remove(&event.window);
    }
}

pub fn resize_render_windows(
    device: Res<RenderDevice>,
    mut windows: ResMut<RenderWindows>,
    mut events: EventReader<WindowResized>,
) {
    for event in events.iter() {
        tracing::trace!(
            "resizing depth texture for {:?} to width = {}, height = {}",
            event.window,
            event.width,
            event.height,
        );

        let Some(window) = windows.windows.get_mut(&event.window) else {
            continue;
        };

        let (depth_texture, depth_texture_view, depth_sampler) =
            create_depth_texture(&device.0, UVec2::new(event.width, event.height));

        window.depth_texture = depth_texture;
        window.depth_texture_view = depth_texture_view;
        window.depth_sampler = depth_sampler;

        window.g_buffer = GBuffer::new(&device.0, event.width, event.height);
    }
}

#[derive(Debug)]
struct GBuffer {
    position: Texture,
    normal: Texture,
    albedo: Texture,
}

impl GBuffer {
    const FORMAT_POSITION: TextureFormat = TextureFormat::Rgba16Float;
    const FORMAT_NORMAL: TextureFormat = TextureFormat::Rgba16Float;
    const FORMAT_ALBEDO: TextureFormat = TextureFormat::Rgba16Float;

    fn new(device: &Device, width: u32, height: u32) -> Self {
        let size = Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        let position = device.create_texture(&TextureDescriptor {
            label: Some("g_buffer_position"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: Self::FORMAT_POSITION,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let normal = device.create_texture(&TextureDescriptor {
            label: Some("g_buffer_normal"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: Self::FORMAT_NORMAL,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let albedo = device.create_texture(&TextureDescriptor {
            label: Some("g_buffer_albedo"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: Self::FORMAT_ALBEDO,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        GBuffer {
            position,
            normal,
            albedo,
        }
    }

    fn targets() -> &'static [Option<ColorTargetState>] {
        &[
            Some(ColorTargetState {
                format: Self::FORMAT_POSITION,
                blend: None,
                write_mask: ColorWrites::ALL,
            }),
            Some(ColorTargetState {
                format: Self::FORMAT_NORMAL,
                blend: None,
                write_mask: ColorWrites::ALL,
            }),
            Some(ColorTargetState {
                format: Self::FORMAT_ALBEDO,
                blend: None,
                write_mask: ColorWrites::ALL,
            }),
        ]
    }
}

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
pub(crate) struct LightUniform {
    pub color: [f32; 3],
    pub _pad0: u32,
    pub position: [f32; 3],
    pub _pad1: u32,
}
