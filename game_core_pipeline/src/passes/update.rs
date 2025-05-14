use std::collections::HashMap;
use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use crossbeam::channel::Receiver;
use game_common::components::Transform;
use game_render::api::{
    BindingResource, BufferInitDescriptor, CommandQueue, DescriptorSet, DescriptorSetDescriptor,
    DescriptorSetEntry, DescriptorSetLayout, TextureRegion, TextureViewDescriptor,
};
use game_render::backend::allocator::UsageFlags;
use game_render::backend::{
    BufferUsage, ImageDataLayout, TextureDescriptor, TextureUsage, max_mips_2d,
};
use game_render::buffer::GpuBuffer;
use game_render::buffer::slab::{SlabBuffer, SlabIndex};
use game_render::graph::{Node, RenderContext};
use game_render::mipmap::MipMapGenerator;
use game_render::texture::image::MipImage;
use game_tracing::trace_span;
use glam::{Mat3, Mat4, UVec2, Vec4};
use parking_lot::Mutex;

use crate::StandardMaterial;
use crate::entities::{Event, ImageId};
use crate::lights::{DirectionalLightUniform, Light, PointLightUniform, SpotLightUniform};

use super::{
    DefaultTextures, MaterialFlags, ObjectState, RawMaterialData, RawObjectData, SceneData, State,
    TextureSlab, TextureSlabIndex,
};

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

        while let Ok(event) = self.events.try_recv() {
            match event {
                Event::CreateScene(id) => {
                    state.scenes.insert(id, SceneData::new());
                }
                Event::DestroyScene(id) => {
                    state.scenes.remove(&id);
                }
                Event::CreateMesh(id, mesh) => {
                    let key = state.mesh.create_mesh(ctx.queue, &mesh);
                    state.meshes.insert(id, key);
                }
                Event::DestroyMesh(id) => {
                    if let Some(key) = state.meshes.remove(&id) {
                        state.mesh.remove(key);
                    }
                }
                Event::CreateImage(id, image) => {
                    let texture =
                        create_image(ctx.queue, &mut state.textures, &self.mipgen, &image);
                    state.images.insert(id, texture);
                }
                Event::DestroyImage(id) => {
                    if let Some(index) = state.images.remove(&id) {
                        state.textures.remove(index);
                    }
                }
                Event::CreateMaterial(id, material) => {
                    let state = &mut *state;
                    let material = create_material(
                        &mut state.material_slab,
                        &state.default_textures,
                        &state.images,
                        material,
                    );
                    state.materials.insert(id, material);
                }
                Event::DestroyMaterial(id) => {
                    if let Some(index) = state.materials.remove(&id) {
                        state.material_slab.remove(index);
                    }
                }
                Event::CreateObject(id, object) => {
                    let state = &mut *state;

                    let Some(material) = state.materials.get(&object.material.id()).copied() else {
                        continue;
                    };

                    let Some(mesh) = state.meshes.get(&object.mesh.id()).copied() else {
                        continue;
                    };

                    if let Some(scene) = state.scenes.get_mut(&object.scene.id()) {
                        let key = state.mesh.create_instance(object.transform, mesh, material);
                        scene.objects.insert(id, ObjectState { id: key });
                    }
                }
                Event::DestroyObject(id) => {
                    let state = &mut *state;
                    for scene in state.scenes.values_mut() {
                        if let Some(instance) = scene.objects.remove(&id) {
                            state.mesh.remove_instance(instance.id);
                        }
                    }
                }
                Event::CreateCamera(id, camera) => {
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

                    let index = scene
                        .directional_lights_buffer
                        .insert(&DirectionalLightUniform::new(&light));
                    scene.directional_lights.insert(id, index);
                }
                Event::CreateLight(id, Light::Point(light)) => {
                    let Some(scene) = state.scenes.get_mut(&light.scene.id()) else {
                        continue;
                    };

                    let index = scene
                        .point_lights_buffer
                        .insert(&PointLightUniform::new(&light));
                    scene.point_lights.insert(id, index);
                }
                Event::CreateLight(id, Light::Spot(light)) => {
                    let Some(scene) = state.scenes.get_mut(&light.scene.id()) else {
                        continue;
                    };

                    let index = scene
                        .spot_lights_buffer
                        .insert(&SpotLightUniform::new(&light));
                    scene.spot_lights.insert(id, index);
                }
                Event::DestroyLight(scene, id) => {
                    let Some(scene) = state.scenes.get_mut(&scene) else {
                        continue;
                    };

                    if let Some(index) = scene.directional_lights.remove(&id) {
                        scene.directional_lights_buffer.remove(index);
                    }

                    if let Some(index) = scene.spot_lights.remove(&id) {
                        scene.spot_lights_buffer.remove(index);
                    }

                    if let Some(index) = scene.spot_lights.remove(&id) {
                        scene.spot_lights_buffer.remove(index);
                    }
                }
            }
        }
    }
}

fn create_image(
    queue: &mut CommandQueue<'_>,
    textures: &mut TextureSlab,
    mipgen: &MipMapGenerator,
    image: &MipImage,
) -> TextureSlabIndex {
    let _span = trace_span!("create_image").entered();

    let supported_usages = queue.supported_texture_usages(image.format());
    if !supported_usages.contains(TextureUsage::TRANSFER_DST)
        || !supported_usages.contains(TextureUsage::TEXTURE_BINDING)
    {
        // TODO: Error handling
        todo!("unsupported texture format: {:?}", image.format());
    }

    // Generate mipmaps for the texture if
    // - we don't already have mipmaps (assume that if we have more than 1 mip
    // we have enough mips.)
    // TODO: In the future we may want to still generate missing mips.
    // - The format can be used to create mipmaps.
    let generate_mipmaps =
        image.mip_levels() == 1 && supported_usages.contains(TextureUsage::RENDER_ATTACHMENT);

    let mut mip_levels = image.mip_levels();
    let mut usage = TextureUsage::TRANSFER_DST | TextureUsage::TEXTURE_BINDING;
    if generate_mipmaps {
        mip_levels = max_mips_2d(UVec2::new(image.root().width(), image.root().height()));
        usage |= TextureUsage::RENDER_ATTACHMENT;
    }

    let texture = queue.create_texture(&TextureDescriptor {
        size: UVec2::new(image.root().width(), image.root().height()),
        mip_levels,
        format: image.format(),
        usage,
    });

    for (mip_level, mip) in image.mips().iter().enumerate() {
        let bytes_per_row = u32::max(1, mip.width() / 4) * mip.format().bytes_per_block();
        let rows_per_image = u32::max(1, mip.height() / 4);

        queue.write_texture(
            TextureRegion {
                texture: &texture,
                mip_level: mip_level as u32,
            },
            mip.as_bytes(),
            ImageDataLayout {
                bytes_per_row,
                rows_per_image,
            },
        );
    }

    if generate_mipmaps {
        mipgen.generate_mipmaps(queue, &texture);
    }

    let view = texture.create_view(&TextureViewDescriptor::default());
    textures.insert(view)
}

fn create_material(
    materials: &mut SlabBuffer<RawMaterialData>,
    default_textures: &DefaultTextures,
    images: &HashMap<ImageId, TextureSlabIndex>,
    material: StandardMaterial,
) -> SlabIndex {
    let _span = trace_span!("create_material").entered();

    let mut flags = MaterialFlags::empty();
    if material.unlit {
        flags |= MaterialFlags::UNLIT;
    }

    if material.flip_normal_y {
        flags |= MaterialFlags::FLIP_NORMAL_Y;
    }

    if material.two_component_normal_encoding {
        flags |= MaterialFlags::NORMAL_ENCODING_TWO_CHANNEL;
    }

    if material.metallic_roughness_texture.is_none()
        && material.specular_glossiness_texture.is_some()
    {
        flags |= MaterialFlags::METALLIC_FROM_SPECULAR;
        flags |= MaterialFlags::ROUGHNESS_FROM_GLOSSINESS;
    }

    let base_color = match material.base_color_texture {
        Some(handle) => match images.get(&handle.id()).copied() {
            Some(texture) => texture,
            None => {
                tracing::warn!("missing base color image: {:?}", handle);
                default_textures.base_color
            }
        },
        None => default_textures.base_color,
    };

    let normal = match material.normal_texture {
        Some(handle) => match images.get(&handle.id()).copied() {
            Some(texture) => texture,
            None => {
                tracing::warn!("missing normal image: {:?}", handle);
                default_textures.normal
            }
        },
        None => default_textures.normal,
    };

    let metallic_roughness = match material.metallic_roughness_texture {
        Some(handle) => match images.get(&handle.id()).copied() {
            Some(texture) => texture,
            None => {
                tracing::warn!("missing metallic/roughness image: {:?}", handle);
                default_textures.metallic_roughness
            }
        },
        None => default_textures.metallic_roughness,
    };

    let specular_glossiness = match material.specular_glossiness_texture {
        Some(handle) => match images.get(&handle.id()).copied() {
            Some(texture) => texture,
            None => {
                tracing::warn!("missing specular/glossiness image: {:?}", handle);
                default_textures.specular_glossiness
            }
        },
        None => default_textures.specular_glossiness,
    };

    materials.insert(&RawMaterialData {
        flags,
        _pad0: [0; 3],
        base_color: material.base_color.as_rgba(),
        roughness: material.roughness,
        metallic: material.metallic,
        reflectance: material.reflectance,
        specular_strength: material.specular_strength,
        specular_color: material.specular_color.as_rgba(),
        base_color_texture_index: base_color,
        normal_texture_index: normal,
        metallic_roughness_texture_index: metallic_roughness,
        specular_glossiness_texture_index: specular_glossiness,
    })
}

fn create_object(
    queue: &mut CommandQueue<'_>,
    layout: &DescriptorSetLayout,
    transform: Transform,
    material: SlabIndex,
    meshlet_offset: u32,
) -> DescriptorSet {
    let _span = trace_span!("create_object_transform").entered();

    let transform_buffer = queue.create_buffer_init(&BufferInitDescriptor {
        contents: bytemuck::bytes_of(&RawObjectData {
            transform: TransformUniform::from(transform),
            meshlet_offset,
            material,
            _pad0: [0; 2],
        }),
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

impl GpuBuffer for TransformUniform {
    const SIZE: usize = size_of::<Self>();
    const ALIGN: usize = 16;
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
