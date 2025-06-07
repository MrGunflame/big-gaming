use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use game_common::components::Color;
use game_tracing::trace_span;
use glam::UVec2;
use parking_lot::Mutex;

use crate::api::{
    BindingResource, Buffer, BufferInitDescriptor, CommandQueue, DepthStencilAttachment,
    DescriptorSet, DescriptorSetDescriptor, DescriptorSetEntry, DescriptorSetLayout,
    RenderPassColorAttachment, RenderPassDescriptor, Sampler, Texture, TextureRegion,
    TextureViewDescriptor,
};
use crate::backend::allocator::UsageFlags;
use crate::backend::{
    max_mips_2d, BufferUsage, ImageDataLayout, IndexFormat, LoadOp, ShaderStages, StoreOp,
    TextureDescriptor, TextureFormat, TextureUsage,
};
use crate::buffer::{DynamicBuffer, IndexBuffer};
use crate::camera::{Camera, CameraUniform, RenderTarget};
use crate::entities::pool::Viewer;
use crate::entities::{
    CameraId, DirectionalLightId, Event, ImageId, MaterialId, MeshId, ObjectId, PointLightId,
    Resources, SceneId, SpotLightId,
};
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
    pub depth_stencils: Mutex<HashMap<RenderTarget, Texture>>,
    pub dst: SlotLabel,
    mipmap_generator: MipMapGenerator,
}

impl ForwardPass {
    pub(super) fn new(
        queue: &mut CommandQueue<'_>,
        forward: Arc<ForwardPipeline>,
        dst: SlotLabel,
    ) -> Self {
        Self {
            state: Mutex::new(ForwardState::new(queue)),
            forward,
            depth_stencils: Mutex::default(),
            dst,
            mipmap_generator: MipMapGenerator::new(queue),
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
                &mut ctx.queue,
                &self.forward.mesh_bind_group_layout,
                &self.forward.material_bind_group_layout,
                &self.forward.vs_bind_group_layout,
                &self.forward.sampler,
                &self.mipmap_generator,
            );
        }

        let size = ctx.read::<Texture>(SlotLabel::SURFACE).unwrap().size();

        for camera in state.cameras.values() {
            if camera.target == ctx.render_target {
                self.update_depth_stencil(ctx.render_target, size, &ctx.queue);

                let scene = state.scenes.get(&camera.scene).unwrap();
                self.render_camera_target(&state, &scene, camera, ctx, size);
                return;
            }
        }

        // Some APIs don't play nicely when not submitting any work
        // for the surface, so we just clear the surface color.
        clear_pass(ctx, self.dst);
    }
}

impl ForwardPass {
    fn update_depth_stencil(&self, target: RenderTarget, size: UVec2, queue: &CommandQueue<'_>) {
        let mut depth_stencils = self.depth_stencils.lock();

        if let Some(texture) = depth_stencils.get(&target) {
            // Texture size unchanged.
            if texture.size() == size {
                return;
            }
        }

        let texture = queue.create_texture(&TextureDescriptor {
            size,
            mip_levels: 1,
            format: TextureFormat::Depth32Float,
            usage: TextureUsage::RENDER_ATTACHMENT,
        });

        depth_stencils.insert(target, texture);
    }

    fn render_camera_target(
        &self,
        state: &ForwardState,
        scene: &Scene,
        camera: &Camera,
        ctx: &mut RenderContext<'_, '_>,
        size: UVec2,
    ) {
        let _span = trace_span!("ForwardPass::render_camera_target").entered();

        let pipeline = &self.forward;
        let pipeline_ref = pipeline
            .pipeline
            .get(&mut ctx.queue, TextureFormat::Rgba16Float);
        let depth_stencils = self.depth_stencils.lock();

        let light_bind_group = ctx.queue.create_descriptor_set(&DescriptorSetDescriptor {
            layout: &pipeline.lights_bind_group_layout,
            entries: &[
                DescriptorSetEntry {
                    binding: 0,
                    resource: BindingResource::Buffer(&scene.directional_lights_buffer),
                },
                DescriptorSetEntry {
                    binding: 1,
                    resource: BindingResource::Buffer(&scene.point_lights_buffer),
                },
                DescriptorSetEntry {
                    binding: 2,
                    resource: BindingResource::Buffer(&scene.spot_lights_buffer),
                },
            ],
        });

        let depth_stencil = depth_stencils.get(&ctx.render_target).unwrap();

        let render_target = ctx.queue.create_texture(&TextureDescriptor {
            size,
            mip_levels: 1,
            format: TextureFormat::Rgba16Float,
            usage: TextureUsage::TEXTURE_BINDING | TextureUsage::RENDER_ATTACHMENT,
        });
        // let target_view = render_target.create_view(&TextureViewDescriptor::default());

        let mut render_pass = ctx.queue.run_render_pass(&RenderPassDescriptor {
            name: "Forward Pass",
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

        // let mut render_pass = ctx.encoder.begin_render_pass(&RenderPassDescriptor {
        //     label: Some("render_pass"),
        //     color_attachments: &[Some(RenderPassColorAttachment {
        //         view: &target_view,
        //         resolve_target: None,
        //         ops: Operations {
        //             load: LoadOp::Clear(Color::BLACK),
        //             store: StoreOp::Store,
        //         },
        //     })],
        //     depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
        //         view: &depth_stencil.view,
        //         depth_ops: Some(Operations {
        //             load: LoadOp::Clear(1.0),
        //             store: StoreOp::Store,
        //         }),
        //         stencil_ops: None,
        //     }),
        //     timestamp_writes: None,
        //     occlusion_query_set: None,
        // });

        let mut push_constants = [0; 84];
        push_constants[0..80].copy_from_slice(bytemuck::bytes_of(&CameraUniform::new(
            camera.transform,
            camera.projection,
        )));
        push_constants[80..84].copy_from_slice(bytemuck::bytes_of(&MainPassOptionsEncoded::new(
            &state.options,
        )));

        render_pass.set_pipeline(&pipeline_ref);
        render_pass.set_push_constants(
            ShaderStages::VERTEX | ShaderStages::FRAGMENT,
            0,
            &push_constants,
        );

        for id in scene.objects.iter() {
            let (mesh, material, transform_bg) = state.objects.get(id).unwrap();

            let (mesh_bg, index_buffer) = state.meshes.get(mesh).unwrap();
            let material_bg = state.materials.get(material).unwrap();

            render_pass.set_descriptor_set(0, transform_bg);
            render_pass.set_descriptor_set(1, mesh_bg);
            render_pass.set_descriptor_set(2, material_bg);
            render_pass.set_descriptor_set(3, &light_bind_group);

            render_pass.set_index_buffer(&index_buffer.buffer, index_buffer.format);
            render_pass.draw_indexed(0..index_buffer.len, 0, 0..1);
        }

        drop(render_pass);
        ctx.write(self.dst, render_target).unwrap();
    }
}

fn clear_pass(ctx: &mut RenderContext<'_, '_>, dst: SlotLabel) {
    let texture = ctx.queue.create_texture(&TextureDescriptor {
        size: UVec2::ONE,
        mip_levels: 1,
        format: TextureFormat::Rgba16Float,
        usage: TextureUsage::TEXTURE_BINDING | TextureUsage::RENDER_ATTACHMENT,
    });

    ctx.queue.run_render_pass(&RenderPassDescriptor {
        name: "Forward Pass",
        color_attachments: &[RenderPassColorAttachment {
            target: &texture.create_view(&TextureViewDescriptor::default()),
            load_op: LoadOp::Clear(Color::BLACK),
            store_op: StoreOp::Store,
        }],
        depth_stencil_attachment: None,
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
    fn new(queue: &mut CommandQueue<'_>) -> Self {
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
            let texture = queue.create_texture(&TextureDescriptor {
                size: UVec2::splat(1),
                mip_levels: 1,
                format,
                usage: TextureUsage::TRANSFER_DST | TextureUsage::TEXTURE_BINDING,
            });

            queue.write_texture(
                TextureRegion {
                    texture: &texture,
                    mip_level: 0,
                },
                &data,
                ImageDataLayout {
                    bytes_per_row: 4,
                    rows_per_image: 1,
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

    meshes: HashMap<MeshId, (DescriptorSet, IndexBuffer)>,
    images: HashMap<ImageId, Texture>,
    materials: HashMap<MaterialId, DescriptorSet>,

    cameras: HashMap<CameraId, Camera>,
    objects: HashMap<ObjectId, (MeshId, MaterialId, DescriptorSet)>,

    scenes: HashMap<SceneId, Scene>,
    options: MainPassOptions,
}

#[derive(Debug)]
struct Scene {
    directional_lights_buffer: Buffer,
    point_lights_buffer: Buffer,
    spot_lights_buffer: Buffer,
    objects: HashSet<ObjectId>,
    cameras: HashSet<CameraId>,
    directional_lights: HashSet<DirectionalLightId>,
    point_lights: HashSet<PointLightId>,
    spot_lights: HashSet<SpotLightId>,
}

impl Scene {
    fn new(queue: &CommandQueue<'_>) -> Self {
        let directional_lights = queue.create_buffer_init(&BufferInitDescriptor {
            contents: DynamicBuffer::<DirectionalLightUniform>::new().as_bytes(),
            usage: BufferUsage::STORAGE,
            flags: UsageFlags::empty(),
        });

        let point_lights = queue.create_buffer_init(&BufferInitDescriptor {
            contents: DynamicBuffer::<PointLightUniform>::new().as_bytes(),
            usage: BufferUsage::STORAGE,
            flags: UsageFlags::empty(),
        });

        let spot_lights = queue.create_buffer_init(&BufferInitDescriptor {
            contents: DynamicBuffer::<SpotLightUniform>::new().as_bytes(),
            usage: BufferUsage::STORAGE,
            flags: UsageFlags::empty(),
        });

        Self {
            directional_lights_buffer: directional_lights,
            point_lights_buffer: point_lights,
            spot_lights_buffer: spot_lights,
            objects: HashSet::new(),
            cameras: HashSet::new(),
            directional_lights: HashSet::new(),
            point_lights: HashSet::new(),
            spot_lights: HashSet::new(),
        }
    }

    /// Returns `true` if the `Scene` contains no entities.
    fn is_empty(&self) -> bool {
        self.objects.is_empty()
            && self.cameras.is_empty()
            && self.directional_lights.is_empty()
            && self.point_lights.is_empty()
            && self.spot_lights.is_empty()
    }
}

impl ForwardState {
    fn new(queue: &mut CommandQueue<'_>) -> Self {
        Self {
            default_textures: DefaultTextures::new(queue),
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
        queue: &CommandQueue<'_>,
        mesh_bind_group_layout: &DescriptorSetLayout,
        material_bind_group_layout: &DescriptorSetLayout,
        object_bind_group_layout: &DescriptorSetLayout,
        material_sampler: &Sampler,
        mipmap_generator: &MipMapGenerator,
    ) {
        let meshes = unsafe { resources.meshes.viewer() };
        let images = unsafe { resources.images.viewer() };
        let materials = unsafe { resources.materials.viewer() };
        let cameras = unsafe { resources.cameras.viewer() };
        let objects = unsafe { resources.objects.viewer() };
        let directional_lights = unsafe { resources.directional_lights.viewer() };
        let point_lights = unsafe { resources.point_lights.viewer() };
        let spot_lights = unsafe { resources.spot_lights.viewer() };

        let mut delete_scenes = Vec::new();

        for event in events.drain(..) {
            match event {
                Event::CreateCamera(id) => {
                    let camera = cameras.get(id.0).unwrap();
                    self.cameras.insert(id, *camera);

                    let scene = self
                        .scenes
                        .entry(camera.scene)
                        .or_insert_with(|| Scene::new(queue));
                    scene.cameras.insert(id);
                }
                Event::DestroyCamera(id) => {
                    self.cameras.remove(&id);

                    for scene in self.scenes.values_mut() {
                        scene.cameras.remove(&id);
                    }
                }
                Event::CreateObject(id) => {
                    let object = objects.get(id.0).unwrap();

                    // If we already uploaded the mesh we can reuse it.
                    // Otherwise we will have to upload it.
                    self.meshes.entry(object.mesh).or_insert_with(|| {
                        let mesh = meshes.get(object.mesh.0).unwrap();
                        upload_mesh(queue, mesh, mesh_bind_group_layout)
                    });

                    self.materials.entry(object.material).or_insert_with(|| {
                        let material = materials.get(object.material.0).unwrap();
                        create_material(
                            queue,
                            material_bind_group_layout,
                            &self.default_textures,
                            &mut self.images,
                            &images,
                            material,
                            material_sampler,
                            mipmap_generator,
                        )
                    });

                    let transform_buffer = queue.create_buffer_init(&BufferInitDescriptor {
                        contents: bytemuck::bytes_of(&TransformUniform::from(object.transform)),
                        usage: BufferUsage::UNIFORM,
                        flags: UsageFlags::empty(),
                    });

                    let object_bind_group = queue.create_descriptor_set(&DescriptorSetDescriptor {
                        layout: object_bind_group_layout,
                        entries: &[DescriptorSetEntry {
                            binding: 0,
                            resource: BindingResource::Buffer(&transform_buffer),
                        }],
                    });

                    self.objects
                        .insert(id, (object.mesh, object.material, object_bind_group));

                    let scene = self
                        .scenes
                        .entry(object.scene)
                        .or_insert_with(|| Scene::new(queue));
                    scene.objects.insert(id);
                }
                Event::DestroyObject(id) => {
                    self.objects.remove(&id);

                    // Remove the object from all scenes.
                    for (scene_id, scene) in &mut self.scenes {
                        scene.objects.remove(&id);

                        if scene.is_empty() {
                            delete_scenes.push(*scene_id);
                        }
                    }
                }
                Event::CreateDirectionalLight(id) => {
                    let light = directional_lights.get(id.0).unwrap();

                    let scene = self
                        .scenes
                        .entry(light.scene)
                        .or_insert_with(|| Scene::new(queue));

                    scene.directional_lights.insert(id);

                    let scene_id = light.scene;
                    let buffer = directional_lights
                        .iter()
                        .copied()
                        .filter(|light| light.scene == scene_id)
                        .map(DirectionalLightUniform::from)
                        .collect::<DynamicBuffer<DirectionalLightUniform>>();

                    let buffer = queue.create_buffer_init(&BufferInitDescriptor {
                        contents: buffer.as_bytes(),
                        usage: BufferUsage::STORAGE,
                        flags: UsageFlags::empty(),
                    });

                    scene.directional_lights_buffer = buffer;
                }
                Event::DestroyDirectionalLight(id) => {
                    for (scene_id, scene) in &mut self.scenes {
                        if !scene.directional_lights.remove(&id) {
                            continue;
                        }

                        if scene.is_empty() {
                            delete_scenes.push(*scene_id);
                            continue;
                        }

                        let buffer = directional_lights
                            .iter()
                            .copied()
                            .filter(|light| light.scene == *scene_id)
                            .map(DirectionalLightUniform::from)
                            .collect::<DynamicBuffer<DirectionalLightUniform>>();

                        let buffer = queue.create_buffer_init(&BufferInitDescriptor {
                            contents: buffer.as_bytes(),
                            usage: BufferUsage::STORAGE,
                            flags: UsageFlags::empty(),
                        });

                        scene.directional_lights_buffer = buffer;
                    }
                }
                Event::CreatePointLight(id) => {
                    let light = point_lights.get(id.0).unwrap();

                    let scene = self
                        .scenes
                        .entry(light.scene)
                        .or_insert_with(|| Scene::new(queue));

                    scene.point_lights.insert(id);

                    let scene_id = light.scene;
                    let buffer = point_lights
                        .iter()
                        .copied()
                        .filter(|light| light.scene == scene_id)
                        .map(PointLightUniform::from)
                        .collect::<DynamicBuffer<PointLightUniform>>();

                    let buffer = queue.create_buffer_init(&BufferInitDescriptor {
                        contents: buffer.as_bytes(),
                        usage: BufferUsage::STORAGE,
                        flags: UsageFlags::empty(),
                    });

                    scene.point_lights_buffer = buffer;
                }
                Event::DestroyPointLight(id) => {
                    for (scene_id, scene) in &mut self.scenes {
                        if !scene.point_lights.remove(&id) {
                            continue;
                        }

                        if scene.is_empty() {
                            delete_scenes.push(*scene_id);
                            continue;
                        }

                        let buffer = point_lights
                            .iter()
                            .copied()
                            .filter(|light| light.scene == *scene_id)
                            .map(PointLightUniform::from)
                            .collect::<DynamicBuffer<PointLightUniform>>();

                        let buffer = queue.create_buffer_init(&BufferInitDescriptor {
                            contents: buffer.as_bytes(),
                            usage: BufferUsage::STORAGE,
                            flags: UsageFlags::empty(),
                        });

                        scene.point_lights_buffer = buffer;
                    }
                }
                Event::CreateSpotLight(id) => {
                    let light = spot_lights.get(id.0).unwrap();

                    let scene = self
                        .scenes
                        .entry(light.scene)
                        .or_insert_with(|| Scene::new(queue));

                    scene.spot_lights.insert(id);

                    let scene_id = light.scene;
                    let buffer = spot_lights
                        .iter()
                        .copied()
                        .filter(|light| light.scene == scene_id)
                        .map(SpotLightUniform::from)
                        .collect::<DynamicBuffer<SpotLightUniform>>();

                    let buffer = queue.create_buffer_init(&BufferInitDescriptor {
                        contents: buffer.as_bytes(),
                        usage: BufferUsage::STORAGE,
                        flags: UsageFlags::empty(),
                    });

                    scene.spot_lights_buffer = buffer;
                }
                Event::DestroySpotLight(id) => {
                    for (scene_id, scene) in &mut self.scenes {
                        if !scene.spot_lights.remove(&id) {
                            continue;
                        }

                        if scene.is_empty() {
                            delete_scenes.push(*scene_id);
                            continue;
                        }

                        let buffer = spot_lights
                            .iter()
                            .copied()
                            .filter(|light| light.scene == *scene_id)
                            .map(SpotLightUniform::from)
                            .collect::<DynamicBuffer<SpotLightUniform>>();

                        let buffer = queue.create_buffer_init(&BufferInitDescriptor {
                            contents: buffer.as_bytes(),
                            usage: BufferUsage::STORAGE,
                            flags: UsageFlags::empty(),
                        });

                        scene.spot_lights_buffer = buffer;
                    }
                }
                Event::UpdateMainPassOptions(options) => {
                    self.options = options;
                }
            }
        }

        for scene in delete_scenes {
            self.scenes.remove(&scene);
        }
    }
}

fn upload_mesh(
    queue: &CommandQueue<'_>,
    mesh: &Mesh,
    bind_group_layout: &DescriptorSetLayout,
) -> (DescriptorSet, IndexBuffer) {
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
            let buffer = queue.create_buffer_init(&BufferInitDescriptor {
                contents: bytemuck::must_cast_slice(&indices),
                usage: BufferUsage::INDEX,
                flags: UsageFlags::empty(),
            });

            IndexBuffer {
                buffer,
                format: IndexFormat::U32,
                len: indices.len() as u32,
            }
        }
        Some(Indices::U16(indices)) => {
            let buffer = queue.create_buffer_init(&BufferInitDescriptor {
                contents: bytemuck::must_cast_slice(&indices),
                usage: BufferUsage::INDEX,
                flags: UsageFlags::empty(),
            });

            IndexBuffer {
                buffer,
                format: IndexFormat::U16,
                len: indices.len() as u32,
            }
        }
        None => todo!(),
    };

    let positions = queue.create_buffer_init(&BufferInitDescriptor {
        contents: bytemuck::must_cast_slice(mesh.positions()),
        usage: BufferUsage::STORAGE,
        flags: UsageFlags::empty(),
    });

    let normals = queue.create_buffer_init(&BufferInitDescriptor {
        contents: bytemuck::must_cast_slice(mesh.normals()),
        usage: BufferUsage::STORAGE,
        flags: UsageFlags::empty(),
    });

    let tangents = queue.create_buffer_init(&BufferInitDescriptor {
        contents: bytemuck::must_cast_slice(mesh.tangents()),
        usage: BufferUsage::STORAGE,
        flags: UsageFlags::empty(),
    });

    let uvs = queue.create_buffer_init(&BufferInitDescriptor {
        contents: bytemuck::must_cast_slice(mesh.uvs()),
        usage: BufferUsage::STORAGE,
        flags: UsageFlags::empty(),
    });

    let bind_group = queue.create_descriptor_set(&DescriptorSetDescriptor {
        layout: bind_group_layout,
        entries: &[
            DescriptorSetEntry {
                binding: 0,
                resource: BindingResource::Buffer(&positions),
            },
            DescriptorSetEntry {
                binding: 1,
                resource: BindingResource::Buffer(&normals),
            },
            DescriptorSetEntry {
                binding: 2,
                resource: BindingResource::Buffer(&tangents),
            },
            DescriptorSetEntry {
                binding: 3,
                resource: BindingResource::Buffer(&uvs),
            },
        ],
    });

    (bind_group, indices)
}

fn create_material(
    queue: &CommandQueue<'_>,
    bind_group_layout: &DescriptorSetLayout,
    default_textures: &DefaultTextures,
    bound_textures: &mut HashMap<ImageId, Texture>,
    images: &Viewer<'_, Image>,
    material: &PbrMaterial,
    sampler: &Sampler,
    mipmap_generator: &MipMapGenerator,
) -> DescriptorSet {
    let _span = trace_span!("create_material").entered();

    let constants = queue.create_buffer_init(&BufferInitDescriptor {
        contents: bytemuck::bytes_of(&MaterialConstants {
            base_color: material.base_color.as_rgba(),
            base_metallic: material.metallic,
            base_roughness: material.roughness,
            reflectance: material.reflectance,
            _pad: [0; 1],
        }),
        usage: BufferUsage::UNIFORM,
        flags: UsageFlags::empty(),
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
                    let image = upload_material_texture(queue, mipmap_generator, image);
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

    queue.create_descriptor_set(&DescriptorSetDescriptor {
        layout: bind_group_layout,
        entries: &[
            DescriptorSetEntry {
                binding: 0,
                resource: BindingResource::Buffer(&constants),
            },
            DescriptorSetEntry {
                binding: 1,
                resource: BindingResource::Texture(
                    &base_color_texture.create_view(&TextureViewDescriptor::default()),
                ),
            },
            DescriptorSetEntry {
                binding: 2,
                resource: BindingResource::Texture(
                    &normal_texture.create_view(&TextureViewDescriptor::default()),
                ),
            },
            DescriptorSetEntry {
                binding: 3,
                resource: BindingResource::Texture(
                    &metallic_roughness_texture.create_view(&TextureViewDescriptor::default()),
                ),
            },
            DescriptorSetEntry {
                binding: 4,
                resource: BindingResource::Sampler(sampler),
            },
        ],
    })
}

fn upload_material_texture(
    queue: &CommandQueue<'_>,
    mipmap_generator: &MipMapGenerator,
    image: &Image,
) -> Texture {
    let _span = trace_span!("upload_material_texture").entered();

    let texture = queue.create_texture(&TextureDescriptor {
        size: UVec2::new(image.width(), image.height()),
        mip_levels: max_mips_2d(UVec2::new(image.width(), image.height())),
        format: image.format(),
        usage: TextureUsage::TRANSFER_DST
            | TextureUsage::TEXTURE_BINDING
            | TextureUsage::RENDER_ATTACHMENT,
    });

    queue.write_texture(
        TextureRegion {
            texture: &texture,
            mip_level: 0,
        },
        image.as_bytes(),
        ImageDataLayout {
            bytes_per_row: 4 * image.width(),
            rows_per_image: image.height(),
        },
    );

    mipmap_generator.generate_mipmaps(queue, &texture);

    texture
}
