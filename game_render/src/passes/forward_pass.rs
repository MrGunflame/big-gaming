use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use game_tracing::trace_span;
use glam::UVec2;
use parking_lot::Mutex;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindingResource, Buffer,
    BufferUsages, Color, CommandEncoderDescriptor, Device, Extent3d, ImageCopyTexture,
    ImageDataLayout, IndexFormat, LoadOp, Operations, Origin3d, Queue, RenderPassColorAttachment,
    RenderPassDepthStencilAttachment, RenderPassDescriptor, Sampler, ShaderStages, StoreOp,
    Texture, TextureAspect, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
    TextureViewDescriptor,
};

use crate::buffer::{DynamicBuffer, IndexBuffer};
use crate::camera::{Camera, CameraUniform, RenderTarget};
use crate::depth_stencil::DepthData;
use crate::entities::pool::Viewer;
use crate::entities::{CameraId, Event, ImageId, MaterialId, MeshId, ObjectId, Resources, SceneId};
use crate::forward::ForwardPipeline;
use crate::graph::{Node, RenderContext, SlotLabel};
use crate::light::pipeline::{DirectionalLightUniform, PointLightUniform, SpotLightUniform};
use crate::mesh::{Indices, Mesh};
use crate::mipmap::MipMapGenerator;
use crate::options::{MainPassOptions, MainPassOptionsEncoded};
use crate::pbr::material::MaterialConstants;
use crate::pbr::mesh::TransformUniform;
use crate::pbr::PbrMaterial;
use crate::texture::Image;

pub(super) struct ForwardPass {
    pub state: Mutex<ForwardState>,
    pub forward: Arc<ForwardPipeline>,
    pub depth_stencils: Mutex<HashMap<RenderTarget, DepthData>>,
    pub dst: SlotLabel,
}

impl ForwardPass {
    pub(super) fn new(
        device: &Device,
        queue: &Queue,
        forward: Arc<ForwardPipeline>,
        dst: SlotLabel,
    ) -> Self {
        Self {
            state: Mutex::new(ForwardState::new(device, queue)),
            forward,
            depth_stencils: Mutex::default(),
            dst,
        }
    }
}

impl Node for ForwardPass {
    fn render(&self, ctx: &mut RenderContext<'_, '_>) {
        let mut state = self.state.lock();
        unsafe {
            let mut events = self.forward.events.borrow_mut();
            state.update(
                &self.forward.resources,
                &mut events,
                ctx.device,
                ctx.queue,
                &self.forward.mesh_bind_group_layout,
                &self.forward.material_bind_group_layout,
                &self.forward.vs_bind_group_layout,
                &self.forward.sampler,
                ctx.mipmap,
            );
        }

        for camera in state.cameras.values() {
            if camera.target == ctx.render_target {
                self.update_depth_stencil(ctx.render_target, ctx.size, ctx.device);

                let scene = state.scenes.get(&camera.scene).unwrap();
                self.render_camera_target(&state, &scene, camera, ctx);
                return;
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
        state: &ForwardState,
        scene: &Scene,
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
                    resource: scene.directional_lights.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: scene.point_lights.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: scene.spot_lights.as_entire_binding(),
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

        for id in scene.objects.iter() {
            let (mesh, material, transform_bg) = state.objects.get(id).unwrap();

            let (mesh_bg, index_buffer) = state.meshes.get(mesh).unwrap();
            let material_bg = state.materials.get(material).unwrap();

            render_pass.set_bind_group(0, transform_bg, &[]);
            render_pass.set_bind_group(1, mesh_bg, &[]);
            render_pass.set_bind_group(2, material_bg, &[]);
            render_pass.set_bind_group(3, &light_bind_group, &[]);

            render_pass.set_index_buffer(index_buffer.buffer.slice(..), index_buffer.format);
            render_pass.draw_indexed(0..index_buffer.len, 0, 0..1);
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

#[derive(Debug)]
struct DefaultTextures {
    default_base_color: Texture,
    default_normal: Texture,
    default_metallic_roughness: Texture,
}

impl DefaultTextures {
    fn new(device: &Device, queue: &Queue) -> Self {
        let [default_base_color, default_normal, default_metallic_roughness] = [
            (TextureFormat::Rgba8UnormSrgb, [255, 255, 255, 255]),
            (
                TextureFormat::Rgba8Unorm,
                // B channel facing towards local Z.
                [(0.5 * 255.0) as u8, (0.5 * 255.0) as u8, 255, 255],
            ),
            (TextureFormat::Rgba8UnormSrgb, [255, 255, 255, 255]),
        ]
        .map(|(format, data)| {
            let texture = device.create_texture(&TextureDescriptor {
                label: None,
                size: Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format,
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
                view_formats: &[],
            });

            queue.write_texture(
                ImageCopyTexture {
                    texture: &texture,
                    mip_level: 0,
                    origin: Origin3d::ZERO,
                    aspect: TextureAspect::All,
                },
                &data,
                ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(4),
                    rows_per_image: Some(1),
                },
                Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
            );

            texture
        });

        Self {
            default_base_color,
            default_normal,
            default_metallic_roughness,
        }
    }
}

#[derive(Debug)]
struct ForwardState {
    default_textures: DefaultTextures,

    meshes: HashMap<MeshId, (BindGroup, IndexBuffer)>,
    images: HashMap<ImageId, Texture>,
    materials: HashMap<MaterialId, BindGroup>,

    cameras: HashMap<CameraId, Camera>,
    objects: HashMap<ObjectId, (MeshId, MaterialId, BindGroup)>,

    scenes: HashMap<SceneId, Scene>,
    options: MainPassOptions,
}

#[derive(Debug)]
struct Scene {
    directional_lights: Buffer,
    point_lights: Buffer,
    spot_lights: Buffer,
    objects: HashSet<ObjectId>,
}

impl Scene {
    fn new(device: &Device) -> Self {
        let directional_lights = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: DynamicBuffer::<DirectionalLightUniform>::new().as_bytes(),
            usage: BufferUsages::STORAGE,
        });

        let point_lights = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: DynamicBuffer::<PointLightUniform>::new().as_bytes(),
            usage: BufferUsages::STORAGE,
        });

        let spot_lights = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: DynamicBuffer::<SpotLightUniform>::new().as_bytes(),
            usage: BufferUsages::STORAGE,
        });

        Self {
            directional_lights,
            point_lights,
            spot_lights,
            objects: HashSet::new(),
        }
    }
}

impl ForwardState {
    fn new(device: &Device, queue: &Queue) -> Self {
        Self {
            default_textures: DefaultTextures::new(device, queue),
            meshes: HashMap::new(),
            images: HashMap::new(),
            materials: HashMap::new(),
            cameras: HashMap::new(),
            objects: HashMap::new(),
            scenes: HashMap::new(),
            options: MainPassOptions::default(),
        }
    }

    unsafe fn update(
        &mut self,
        resources: &Resources,
        events: &mut Vec<Event>,
        device: &Device,
        queue: &Queue,
        mesh_bind_group_layout: &BindGroupLayout,
        material_bind_group_layout: &BindGroupLayout,
        object_bind_group_layout: &BindGroupLayout,
        material_sampler: &Sampler,
        mipmap_generator: &mut MipMapGenerator,
    ) {
        let meshes = unsafe { resources.meshes.viewer() };
        let images = unsafe { resources.images.viewer() };
        let materials = unsafe { resources.materials.viewer() };
        let cameras = unsafe { resources.cameras.viewer() };
        let objects = unsafe { resources.objects.viewer() };
        let directional_lights = unsafe { resources.directional_lights.viewer() };
        let point_lights = unsafe { resources.point_lights.viewer() };
        let spot_lights = unsafe { resources.spot_lights.viewer() };

        for event in events.drain(..) {
            match event {
                Event::CreateCamera(id) => {
                    let camera = cameras.get(id.0).unwrap();
                    self.cameras.insert(id, *camera);
                }
                Event::DestroyCamera(id) => {
                    self.cameras.remove(&id);
                }
                Event::CreateObject(id) => {
                    let object = objects.get(id.0).unwrap();

                    // If we already uploaded the mesh we can reuse it.
                    // Otherwise we will have to upload it.
                    self.meshes.entry(object.mesh).or_insert_with(|| {
                        let mesh = meshes.get(object.mesh.0).unwrap();
                        upload_mesh(device, mesh, mesh_bind_group_layout)
                    });

                    self.materials.entry(object.material).or_insert_with(|| {
                        let material = materials.get(object.material.0).unwrap();
                        create_material(
                            device,
                            queue,
                            mipmap_generator,
                            material_bind_group_layout,
                            &self.default_textures,
                            &mut self.images,
                            &images,
                            material,
                            material_sampler,
                        )
                    });

                    let transform_buffer = device.create_buffer_init(&BufferInitDescriptor {
                        label: None,
                        contents: bytemuck::bytes_of(&TransformUniform::from(object.transform)),
                        usage: BufferUsages::UNIFORM,
                    });

                    let object_bind_group = device.create_bind_group(&BindGroupDescriptor {
                        label: None,
                        layout: object_bind_group_layout,
                        entries: &[BindGroupEntry {
                            binding: 0,
                            resource: transform_buffer.as_entire_binding(),
                        }],
                    });

                    self.objects
                        .insert(id, (object.mesh, object.material, object_bind_group));

                    let scene = self
                        .scenes
                        .entry(object.scene)
                        .or_insert_with(|| Scene::new(device));
                    scene.objects.insert(id);
                }
                Event::DestroyObject(id) => {
                    self.objects.remove(&id);

                    // Remove the object from all scenes.
                    for scene in self.scenes.values_mut() {
                        scene.objects.remove(&id);
                    }
                }
                Event::CreateDirectionalLight(_) | Event::DestroyDirectionalLight(_) => {
                    for (scene_id, scene) in &mut self.scenes {
                        let buffer = directional_lights
                            .iter()
                            .copied()
                            .filter(|light| light.scene == *scene_id)
                            .map(DirectionalLightUniform::from)
                            .collect::<DynamicBuffer<DirectionalLightUniform>>();

                        let buffer = device.create_buffer_init(&BufferInitDescriptor {
                            label: None,
                            contents: buffer.as_bytes(),
                            usage: BufferUsages::STORAGE,
                        });

                        scene.directional_lights = buffer;
                    }
                }
                Event::CreatePointLight(_) | Event::DestroyPointLight(_) => {
                    for (scene_id, scene) in &mut self.scenes {
                        let buffer = point_lights
                            .iter()
                            .copied()
                            .filter(|light| light.scene == *scene_id)
                            .map(PointLightUniform::from)
                            .collect::<DynamicBuffer<PointLightUniform>>();

                        let buffer = device.create_buffer_init(&BufferInitDescriptor {
                            label: None,
                            contents: buffer.as_bytes(),
                            usage: BufferUsages::STORAGE,
                        });

                        scene.point_lights = buffer;
                    }
                }
                Event::CreateSpotLight(_) | Event::DestroySpotLight(_) => {
                    for (scene_id, scene) in &mut self.scenes {
                        let buffer = spot_lights
                            .iter()
                            .copied()
                            .filter(|light| light.scene == *scene_id)
                            .map(SpotLightUniform::from)
                            .collect::<DynamicBuffer<SpotLightUniform>>();

                        let buffer = device.create_buffer_init(&BufferInitDescriptor {
                            label: None,
                            contents: buffer.as_bytes(),
                            usage: BufferUsages::STORAGE,
                        });

                        scene.spot_lights = buffer;
                    }
                }
                Event::UpdateMainPassOptions(options) => {
                    self.options = options;
                }
            }
        }
    }
}

fn upload_mesh(
    device: &Device,
    mesh: &Mesh,
    bind_group_layout: &BindGroupLayout,
) -> (BindGroup, IndexBuffer) {
    let _span = trace_span!("upload_mesh").entered();
    // FIXME: Since meshes are user controlled, we might not catch invalid
    // meshes with a panic and simply ignore them.
    assert!(!mesh.positions().is_empty());
    assert!(!mesh.normals().is_empty());
    assert!(!mesh.tangents().is_empty());
    assert!(!mesh.uvs().is_empty());
    assert!(!mesh.indicies().as_ref().unwrap().is_empty());

    let indices = match mesh.indicies() {
        Some(Indices::U32(indices)) => {
            let buffer = device.create_buffer_init(&BufferInitDescriptor {
                label: None,
                contents: bytemuck::must_cast_slice(&indices),
                usage: BufferUsages::INDEX,
            });

            IndexBuffer {
                buffer,
                format: IndexFormat::Uint32,
                len: indices.len() as u32,
            }
        }
        Some(Indices::U16(indices)) => {
            let buffer = device.create_buffer_init(&BufferInitDescriptor {
                label: None,
                contents: bytemuck::must_cast_slice(&indices),
                usage: BufferUsages::INDEX,
            });

            IndexBuffer {
                buffer,
                format: IndexFormat::Uint16,
                len: indices.len() as u32,
            }
        }
        None => todo!(),
    };

    let positions = device.create_buffer_init(&BufferInitDescriptor {
        label: None,
        contents: bytemuck::must_cast_slice(mesh.positions()),
        usage: BufferUsages::STORAGE,
    });

    let normals = device.create_buffer_init(&BufferInitDescriptor {
        label: None,
        contents: bytemuck::must_cast_slice(mesh.normals()),
        usage: BufferUsages::STORAGE,
    });

    let tangents = device.create_buffer_init(&BufferInitDescriptor {
        label: None,
        contents: bytemuck::must_cast_slice(mesh.tangents()),
        usage: BufferUsages::STORAGE,
    });

    let uvs = device.create_buffer_init(&BufferInitDescriptor {
        label: None,
        contents: bytemuck::must_cast_slice(mesh.uvs()),
        usage: BufferUsages::STORAGE,
    });

    let bind_group = device.create_bind_group(&BindGroupDescriptor {
        label: None,
        layout: bind_group_layout,
        entries: &[
            BindGroupEntry {
                binding: 0,
                resource: positions.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 1,
                resource: normals.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 2,
                resource: tangents.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 3,
                resource: uvs.as_entire_binding(),
            },
        ],
    });

    (bind_group, indices)
}

fn create_material(
    device: &Device,
    queue: &Queue,
    mipmap_generator: &mut MipMapGenerator,
    bind_group_layout: &BindGroupLayout,
    default_textures: &DefaultTextures,
    bound_textures: &mut HashMap<ImageId, Texture>,
    images: &Viewer<'_, Image>,
    material: &PbrMaterial,
    sampler: &Sampler,
) -> BindGroup {
    let _span = trace_span!("create_material").entered();

    let constants = device.create_buffer_init(&BufferInitDescriptor {
        label: None,
        contents: bytemuck::bytes_of(&MaterialConstants {
            base_color: material.base_color.as_rgba(),
            base_metallic: material.metallic,
            base_roughness: material.roughness,
            reflectance: material.reflectance,
            _pad: [0; 1],
        }),
        usage: BufferUsages::UNIFORM,
    });

    // Ensure all textures exist before we try to access them.
    for id in [
        material.base_color_texture,
        material.normal_texture,
        material.metallic_roughness_texture,
    ] {
        match id {
            Some(id) => {
                if !bound_textures.contains_key(&id) {
                    let image = images.get(id.0).unwrap();
                    let image = upload_material_texture(device, queue, mipmap_generator, image);
                    bound_textures.insert(id, image);
                }
            }
            None => (),
        }
    }

    let base_color_texture = match material.base_color_texture {
        Some(id) => bound_textures.get(&id).unwrap(),
        None => &default_textures.default_base_color,
    };

    let normal_texture = match material.normal_texture {
        Some(id) => bound_textures.get(&id).unwrap(),
        None => &default_textures.default_normal,
    };

    let metallic_roughness_texture = match material.metallic_roughness_texture {
        Some(id) => bound_textures.get(&id).unwrap(),
        None => &default_textures.default_metallic_roughness,
    };

    device.create_bind_group(&BindGroupDescriptor {
        label: None,
        layout: bind_group_layout,
        entries: &[
            BindGroupEntry {
                binding: 0,
                resource: constants.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 1,
                resource: BindingResource::TextureView(
                    &base_color_texture.create_view(&TextureViewDescriptor::default()),
                ),
            },
            BindGroupEntry {
                binding: 2,
                resource: BindingResource::TextureView(
                    &normal_texture.create_view(&TextureViewDescriptor::default()),
                ),
            },
            BindGroupEntry {
                binding: 2,
                resource: BindingResource::TextureView(
                    &metallic_roughness_texture.create_view(&TextureViewDescriptor::default()),
                ),
            },
            BindGroupEntry {
                binding: 3,
                resource: BindingResource::Sampler(sampler),
            },
        ],
    })
}

fn upload_material_texture(
    device: &Device,
    queue: &Queue,
    mipmap_generator: &mut MipMapGenerator,
    image: &Image,
) -> Texture {
    let _span = trace_span!("upload_material_texture").entered();

    let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor { label: None });

    let size = Extent3d {
        width: image.width(),
        height: image.height(),
        depth_or_array_layers: 1,
    };

    let texture = device.create_texture(&TextureDescriptor {
        label: None,
        size,
        mip_level_count: size.max_mips(TextureDimension::D2),
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: image.format(),
        usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
        view_formats: &[],
    });

    queue.write_texture(
        ImageCopyTexture {
            texture: &texture,
            mip_level: 0,
            origin: Origin3d::ZERO,
            aspect: TextureAspect::All,
        },
        image.as_bytes(),
        ImageDataLayout {
            offset: 0,
            // TODO: Support for non-RGBA (non 4 px) textures.
            bytes_per_row: Some(4 * image.width()),
            rows_per_image: Some(image.height()),
        },
        size,
    );

    mipmap_generator.generate_mipmaps(device, &mut encoder, &texture);
    queue.submit(std::iter::once(encoder.finish()));

    texture
}
