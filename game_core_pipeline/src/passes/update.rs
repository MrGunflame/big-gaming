use std::collections::HashMap;
use std::sync::Arc;

use bitflags::bitflags;
use bytemuck::{Pod, Zeroable};
use crossbeam::channel::Receiver;
use game_common::components::Transform;
use game_render::api::{
    BindingResource, BufferInitDescriptor, CommandQueue, DescriptorSet, DescriptorSetDescriptor,
    DescriptorSetEntry, DescriptorSetLayout, Texture, TextureRegion, TextureViewDescriptor,
};
use game_render::backend::allocator::UsageFlags;
use game_render::backend::{
    BufferUsage, ImageDataLayout, IndexFormat, TextureDescriptor, TextureUsage, max_mips_2d,
};
use game_render::buffer::{DynamicBuffer, IndexBuffer};
use game_render::graph::{Node, RenderContext};
use game_render::mesh::{Indices, Mesh};
use game_render::mipmap::MipMapGenerator;
use game_render::texture::Image;
use game_tracing::trace_span;
use glam::{Mat3, Mat4, UVec2, Vec4};
use parking_lot::Mutex;

use crate::StandardMaterial;
use crate::entities::{Event, ImageId};
use crate::lights::{DirectionalLightUniform, Light, PointLightUniform, SpotLightUniform};

use super::{DefaultTextures, MeshState, ObjectState, SceneData, State};

#[derive(Debug)]
pub(super) struct UpdatePass {
    state: Arc<Mutex<State>>,
    events: Receiver<Event>,
    mipgen: MipMapGenerator,
}

impl UpdatePass {
    pub(super) fn new(
        queue: &mut CommandQueue<'_>,
        state: Arc<Mutex<State>>,
        events: Receiver<Event>,
    ) -> Self {
        Self {
            state,
            events,
            mipgen: MipMapGenerator::new(queue),
        }
    }
}

impl Node for UpdatePass {
    fn render<'a>(&self, ctx: &'a mut RenderContext<'_, 'a>) {
        let _span = trace_span!("UpdatePass::render").entered();

        let mut state = self.state.lock();

        let mut directional_lights_changed = false;
        let mut point_lights_changed = false;
        let mut spot_lights_changed = false;

        while let Ok(event) = self.events.try_recv() {
            match event {
                Event::CreateScene(id) => {
                    state.scenes.insert(id, SceneData::new(ctx.queue));
                }
                Event::DestroyScene(id) => {
                    state.scenes.remove(&id);
                }
                Event::CreateMesh(id, mesh) => {
                    let mesh = create_mesh(ctx.queue, &state.mesh_descriptor_layout, &mesh);
                    state.meshes.insert(id, mesh);
                }
                Event::DestroyMesh(id) => {
                    state.meshes.remove(&id);
                }
                Event::CreateImage(id, image) => {
                    let texture = create_image(ctx.queue, &self.mipgen, &image);
                    state.images.insert(id, texture);
                }
                Event::DestroyImage(id) => {
                    state.images.remove(&id);
                }
                Event::CreateMaterial(id, material) => {
                    let material = create_material(
                        ctx.queue,
                        &state.default_textures,
                        &state.images,
                        &state.material_descriptor_layout,
                        material,
                    );
                    state.materials.insert(id, material);
                }
                Event::DestroyMaterial(id) => {
                    state.materials.remove(&id);
                }
                Event::CreateObject(id, object) => {
                    tracing::trace!("create obj");
                    let transform = create_object_transform(
                        ctx.queue,
                        &state.transform_descriptor_layout,
                        object.transform,
                    );
                    if let Some(scene) = state.scenes.get_mut(&object.scene.id()) {
                        scene.objects.insert(
                            id,
                            ObjectState {
                                transform,
                                mesh: object.mesh.id(),
                                material: object.material.id(),
                            },
                        );
                    }
                }
                Event::DestroyObject(id) => {
                    tracing::trace!("rm obj");
                    for scene in state.scenes.values_mut() {
                        scene.objects.remove(&id);
                    }
                }
                Event::CreateCamera(id, camera) => {
                    tracing::trace!("camera");
                    if let Some(scene) = state.scenes.get_mut(&camera.scene.id()) {
                        scene.cameras.insert(id, camera);
                    }
                }
                Event::DestroyCamera(id) => {
                    for scene in state.scenes.values_mut() {
                        scene.cameras.remove(&id);
                    }
                }
                Event::CreateLight(id, Light::Directional(light)) => {
                    let Some(scene) = state.scenes.get_mut(&light.scene.id()) else {
                        continue;
                    };

                    scene.directional_lights.insert(id, light);
                    directional_lights_changed = true;
                }
                Event::CreateLight(id, Light::Point(light)) => {
                    let Some(scene) = state.scenes.get_mut(&light.scene.id()) else {
                        continue;
                    };

                    scene.point_lights.insert(id, light);
                    point_lights_changed = true;
                }
                Event::CreateLight(id, Light::Spot(light)) => {
                    let Some(scene) = state.scenes.get_mut(&light.scene.id()) else {
                        continue;
                    };

                    scene.spot_lights.insert(id, light);
                    spot_lights_changed = true;
                }
                Event::DestroyLight(id) => {
                    for scene in state.scenes.values_mut() {
                        if scene.directional_lights.remove(&id).is_some() {
                            directional_lights_changed = true;
                            break;
                        }

                        if scene.point_lights.remove(&id).is_some() {
                            point_lights_changed = true;
                            break;
                        }

                        if scene.spot_lights.remove(&id).is_some() {
                            spot_lights_changed = true;
                            break;
                        }
                    }
                }
            }
        }

        // Recreate buffers for lights that have been updated.
        for scene in state.scenes.values_mut() {
            if directional_lights_changed {
                let buffer = scene
                    .directional_lights
                    .values()
                    .map(DirectionalLightUniform::new)
                    .collect::<DynamicBuffer<_>>();

                scene.directional_light_buffer =
                    ctx.queue.create_buffer_init(&BufferInitDescriptor {
                        contents: buffer.as_bytes(),
                        usage: BufferUsage::STORAGE,
                        flags: UsageFlags::empty(),
                    })
            }

            if point_lights_changed {
                let buffer = scene
                    .point_lights
                    .values()
                    .map(PointLightUniform::new)
                    .collect::<DynamicBuffer<_>>();

                scene.point_light_buffer = ctx.queue.create_buffer_init(&BufferInitDescriptor {
                    contents: buffer.as_bytes(),
                    usage: BufferUsage::STORAGE,
                    flags: UsageFlags::empty(),
                });
            }

            if spot_lights_changed {
                let buffer = scene
                    .spot_lights
                    .values()
                    .map(SpotLightUniform::new)
                    .collect::<DynamicBuffer<_>>();

                scene.spot_light_buffer = ctx.queue.create_buffer_init(&BufferInitDescriptor {
                    contents: buffer.as_bytes(),
                    usage: BufferUsage::STORAGE,
                    flags: UsageFlags::empty(),
                });
            }
        }
    }
}

fn create_mesh(
    queue: &mut CommandQueue<'_>,
    layout: &DescriptorSetLayout,
    mesh: &Mesh,
) -> MeshState {
    let _span = trace_span!("create_mesh").entered();

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

    let descriptor = queue.create_descriptor_set(&DescriptorSetDescriptor {
        layout,
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

    MeshState {
        descriptor,
        indices,
    }
}

fn create_image(queue: &mut CommandQueue<'_>, mipgen: &MipMapGenerator, image: &Image) -> Texture {
    let _span = trace_span!("create_image").entered();

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

    mipgen.generate_mipmaps(queue, &texture);

    texture
}

fn create_material(
    queue: &mut CommandQueue<'_>,
    default_textures: &DefaultTextures,
    images: &HashMap<ImageId, Texture>,
    layout: &DescriptorSetLayout,
    material: StandardMaterial,
) -> DescriptorSet {
    let _span = trace_span!("create_material").entered();

    let mut flags = MaterialFlags::empty();
    if material.unlit {
        flags |= MaterialFlags::UNLIT;
    }

    if material.flip_normal_y {
        flags |= MaterialFlags::FLIP_NORMAL_Y;
    }

    if material.metallic_roughness_texture.is_none()
        && material.specular_glossiness_texture.is_some()
    {
        flags |= MaterialFlags::METALLIC_FROM_SPECULAR;
        flags |= MaterialFlags::ROUGHNESS_FROM_GLOSSINESS;
    }

    let data = queue.create_buffer_init(&BufferInitDescriptor {
        contents: bytemuck::bytes_of(&RawMaterialData {
            flags,
            base_color: material.base_color.as_rgba(),
            roughness: material.roughness,
            metallic: material.metallic,
            reflectance: material.reflectance,
            specular_color: material.specular_color.as_rgba(),
            specular_strength: material.specular_strength,
            _pad0: [0; 3],
        }),
        usage: BufferUsage::UNIFORM,
        flags: UsageFlags::empty(),
    });

    let base_color = match material.base_color_texture {
        Some(handle) => match images.get(&handle.id()) {
            Some(texture) => texture,
            None => {
                tracing::warn!("missing base color image: {:?}", handle);
                &default_textures.base_color
            }
        },
        None => &default_textures.base_color,
    };

    let normal = match material.normal_texture {
        Some(handle) => match images.get(&handle.id()) {
            Some(texture) => texture,
            None => {
                tracing::warn!("missing normal image: {:?}", handle);
                &default_textures.normal
            }
        },
        None => &default_textures.normal,
    };

    let metallic_roughness = match material.metallic_roughness_texture {
        Some(handle) => match images.get(&handle.id()) {
            Some(texture) => texture,
            None => {
                tracing::warn!("missing metallic/roughness image: {:?}", handle);
                &default_textures.metallic_roughness
            }
        },
        None => &default_textures.metallic_roughness,
    };

    let specular_glossiness = match material.specular_glossiness_texture {
        Some(handle) => match images.get(&handle.id()) {
            Some(texture) => texture,
            None => {
                tracing::warn!("missing specular/glossiness image: {:?}", handle);
                &default_textures.specular_glossiness
            }
        },
        None => &default_textures.specular_glossiness,
    };

    queue.create_descriptor_set(&DescriptorSetDescriptor {
        layout,
        entries: &[
            DescriptorSetEntry {
                binding: 0,
                resource: BindingResource::Buffer(&data),
            },
            DescriptorSetEntry {
                binding: 1,
                resource: BindingResource::Texture(
                    &base_color.create_view(&TextureViewDescriptor::default()),
                ),
            },
            DescriptorSetEntry {
                binding: 2,
                resource: BindingResource::Texture(
                    &normal.create_view(&TextureViewDescriptor::default()),
                ),
            },
            DescriptorSetEntry {
                binding: 3,
                resource: BindingResource::Texture(
                    &metallic_roughness.create_view(&TextureViewDescriptor::default()),
                ),
            },
            DescriptorSetEntry {
                binding: 4,
                resource: BindingResource::Texture(
                    &specular_glossiness.create_view(&TextureViewDescriptor::default()),
                ),
            },
        ],
    })
}

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
struct RawMaterialData {
    flags: MaterialFlags,
    _pad0: [u32; 3],
    base_color: [f32; 4],
    roughness: f32,
    metallic: f32,
    reflectance: f32,
    specular_strength: f32,
    specular_color: [f32; 4],
}

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Zeroable, Pod)]
    #[repr(transparent)]
    struct MaterialFlags: u32 {
        const UNLIT = 1 << 0;
        const FLIP_NORMAL_Y = 1 << 1;
        const METALLIC_FROM_SPECULAR = 1 << 2;
        const ROUGHNESS_FROM_GLOSSINESS = 1 << 3;
    }
}

fn create_object_transform(
    queue: &mut CommandQueue<'_>,
    layout: &DescriptorSetLayout,
    transform: Transform,
) -> DescriptorSet {
    let _span = trace_span!("create_object_transform").entered();

    let transform_buffer = queue.create_buffer_init(&BufferInitDescriptor {
        contents: bytemuck::bytes_of(&TransformUniform::from(transform)),
        usage: BufferUsage::UNIFORM,
        flags: UsageFlags::empty(),
    });

    queue.create_descriptor_set(&DescriptorSetDescriptor {
        layout,
        entries: &[DescriptorSetEntry {
            binding: 0,
            resource: BindingResource::Buffer(&transform_buffer),
        }],
    })
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
