use std::collections::HashMap;

use bytemuck::{Pod, Zeroable};
use game_tracing::trace_span;
use glam::UVec2;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{
    BindGroup, BindGroupDescriptor, Buffer, BufferUsages, Device, Queue, Texture, TextureFormat,
};

use crate::buffer::{DynamicBuffer, GpuBuffer, IndexBuffer};
use crate::camera::{Camera, CameraBuffer};
use crate::entities::{CameraId, DirectionalLightId, Object, ObjectId, PointLightId, SpotLightId};
use crate::forward::ForwardPipeline;
use crate::light::pipeline::{
    update_directional_lights, update_point_lights, update_spot_lights, DirectionalLightUniform,
    PointLightUniform, SpotLightUniform,
};
use crate::light::{DirectionalLight, PointLight, SpotLight};
use crate::mesh::Mesh;
use crate::mipmap::MipMapGenerator;
use crate::pbr::material::{create_texture, MaterialId, Materials};
use crate::pbr::mesh::{update_mesh_bind_group, update_transform_buffer, MeshId, Meshes};
use crate::pbr::PbrMaterial;
use crate::texture::{Image, ImageId, Images};

pub(crate) struct RenderState {
    pub cameras: HashMap<CameraId, Camera>,
    pub camera_buffers: HashMap<CameraId, CameraBuffer>,
    pub objects: HashMap<ObjectId, Object>,
    /// object transform buffers
    pub object_buffers: HashMap<ObjectId, Buffer>,

    pub materials_buffer: Buffer,

    pub directional_lights: HashMap<DirectionalLightId, DirectionalLight>,
    pub point_lights: HashMap<PointLightId, PointLight>,
    pub spot_lights: HashMap<SpotLightId, SpotLight>,
    pub directional_lights_buffer: Buffer,
    pub point_lights_buffer: Buffer,
    pub spot_lights_buffer: Buffer,

    pub events: Vec<Event>,

    pub meshes: HashMap<MeshId, (BindGroup, IndexBuffer)>,

    // New resoures to upload.
    pub meshes_queued: HashMap<MeshId, Mesh>,
    pub materials_queued: HashMap<MaterialId, PbrMaterial>,
    pub images_queued: HashMap<ImageId, Image>,

    pub images: HashMap<ImageId, Texture>,
    pub image_indices: HashMap<ImageId, u32>,
    pub materials: HashMap<MaterialId, MaterialUniform>,

    pub placeholder_texture: Option<Texture>,
}

impl RenderState {
    pub fn new(device: &Device, pipeline: &ForwardPipeline, images: &Images) -> Self {
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

        let buffer = DynamicBuffer::<MaterialUniform>::new();
        let materials_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: buffer.as_bytes(),
            usage: BufferUsages::STORAGE,
        });

        let mut images_queued = HashMap::new();
        for id in [
            pipeline.default_textures.default_base_color_texture,
            pipeline.default_textures.default_normal_texture,
            pipeline.default_textures.default_metallic_roughness_texture,
        ] {
            let img = images.get(id).unwrap().clone();
            images_queued.insert(id, img);
        }

        Self {
            directional_lights_buffer: directional_lights,
            spot_lights_buffer: spot_lights,
            point_lights_buffer: point_lights,
            cameras: HashMap::new(),
            objects: HashMap::new(),
            events: vec![],
            meshes: HashMap::new(),
            materials: HashMap::new(),
            materials_queued: HashMap::new(),
            meshes_queued: HashMap::new(),
            camera_buffers: HashMap::new(),
            object_buffers: HashMap::new(),
            directional_lights: HashMap::new(),
            point_lights: HashMap::new(),
            spot_lights: HashMap::new(),
            materials_buffer,
            images_queued,
            images: HashMap::new(),
            image_indices: HashMap::new(),
            placeholder_texture: None,
        }
    }

    pub fn update(
        &mut self,
        event: Event,
        meshes: &Meshes,
        materials: &Materials,
        images: &Images,
    ) {
        let _span = trace_span!("RenderState::update").entered();

        self.events.push(event);

        match event {
            Event::CreateCamera(id, camera) => {
                self.cameras.insert(id, camera);
            }
            Event::DestroyCamera(id) => {
                self.cameras.remove(&id);
            }
            Event::CreateObject(id, object) => {
                self.objects.insert(id, object);

                // FIXME: What should we if a object is missing the mesh/material/images?
                // For now we panic because such a state would likely be invalid, but skipping
                // or even destroying the entitiy might be a more robust solution.

                if !self.meshes.contains_key(&object.mesh) {
                    if !meshes.contains_key(object.mesh) {
                        tracing::warn!("object {:?} missing mesh {:?}", id, object.mesh);
                    }

                    let mesh = meshes.get(object.mesh).unwrap().clone();
                    self.meshes_queued.insert(object.mesh, mesh);
                }

                if !self.materials.contains_key(&object.material) {
                    let material = *materials.get(object.material).unwrap();
                    self.materials_queued.insert(object.material, material);

                    for id in [
                        material.base_color_texture,
                        material.normal_texture,
                        material.metallic_roughness_texture,
                    ]
                    .into_iter()
                    .flatten()
                    {
                        let img = images.get(id).unwrap().clone();
                        self.images_queued.insert(id, img);
                    }
                }
            }
            Event::DestroyObject(id) => {
                self.objects.remove(&id);
            }
            Event::CreateDirectionalLight(id, light) => {
                self.directional_lights.insert(id, light);
            }
            Event::DestroyDirectionalLight(id) => {
                self.directional_lights.remove(&id);
            }
            Event::CreatePointLight(id, light) => {
                self.point_lights.insert(id, light);
            }
            Event::DestroyPointLight(id) => {
                self.point_lights.remove(&id);
            }
            Event::CreateSpotLight(id, light) => {
                self.spot_lights.insert(id, light);
            }
            Event::DestroySpotLight(id) => {
                self.spot_lights.remove(&id);
            }
        }
    }

    pub fn update_buffers(
        &mut self,
        device: &Device,
        queue: &Queue,
        pipeline: &ForwardPipeline,
        mipmap_generator: &mut MipMapGenerator,
    ) {
        let _span = trace_span!("RenderState::update_buffers").entered();

        for (id, mesh) in self.meshes_queued.drain() {
            let (bg, buf) = update_mesh_bind_group(device, pipeline, &mesh);
            self.meshes.insert(id, (bg, buf));
        }

        let mut rebuild_image_indices = false;
        let mut rebuild_materials = false;

        for (id, image) in self.images_queued.drain() {
            let texture = create_texture(&image, device, queue, mipmap_generator);
            self.images.insert(id, texture);
            rebuild_image_indices = true;
        }

        if self.placeholder_texture.is_none() {
            let image = Image::new(UVec2::ONE, TextureFormat::Rgba8UnormSrgb, vec![0, 0, 0, 0]);
            let texture = create_texture(&image, device, queue, mipmap_generator);
            self.placeholder_texture = Some(texture);
        }

        if rebuild_image_indices {
            self.image_indices.clear();
            for (index, id) in self.images.keys().enumerate() {
                self.image_indices.insert(*id, index as u32);
            }
        }

        for (id, material) in self.materials_queued.drain() {
            let albedo_texture = material
                .base_color_texture
                .unwrap_or(pipeline.default_textures.default_base_color_texture);
            let normal_texture = material
                .normal_texture
                .unwrap_or(pipeline.default_textures.default_normal_texture);
            let metallic_roughness_texture = material
                .metallic_roughness_texture
                .unwrap_or(pipeline.default_textures.default_metallic_roughness_texture);

            let albedo_texture = *self.image_indices.get(&albedo_texture).unwrap();
            let normal_texture = *self.image_indices.get(&normal_texture).unwrap();
            let metallic_roughness_texture =
                *self.image_indices.get(&metallic_roughness_texture).unwrap();

            let material = MaterialUniform {
                color: material.base_color.0,
                metallic: material.metallic,
                roughness: material.roughness,
                reflectance: material.reflectance,
                albedo_texture,
                normal_texture,
                metallic_roughness_texture,
                _pad0: [0; 1],
                _pad1: [0; 1],
            };

            self.materials.insert(id, material);

            rebuild_materials = true;
        }

        if rebuild_materials {
            let mut buffer = DynamicBuffer::<MaterialUniform>::new();
            for material in self.materials.values() {
                buffer.push(*material);
            }

            self.materials_buffer = device.create_buffer_init(&BufferInitDescriptor {
                label: None,
                contents: buffer.as_bytes(),
                usage: BufferUsages::STORAGE,
            });
        }

        let mut rebuild_dir_lights = false;
        let mut rebuild_point_lights = false;
        let mut rebuild_spot_lights = false;

        for event in self.events.drain(..) {
            match event {
                Event::CreateCamera(id, camera) => {
                    let buffer = CameraBuffer::new(
                        camera.transform,
                        camera.projection,
                        device,
                        camera.target,
                    );

                    self.camera_buffers.insert(id, buffer);
                }
                Event::DestroyCamera(id) => {
                    self.cameras.remove(&id);
                    self.camera_buffers.remove(&id);
                }
                Event::CreateObject(id, object) => {
                    let buffer = update_transform_buffer(object.transform, device);
                    self.object_buffers.insert(id, buffer);
                }
                Event::DestroyObject(id) => {
                    self.objects.remove(&id);
                    self.object_buffers.remove(&id);
                }
                Event::CreateDirectionalLight(_, _) | Event::DestroyDirectionalLight(_) => {
                    rebuild_dir_lights = true;
                }
                Event::CreatePointLight(_, _) | Event::DestroyPointLight(_) => {
                    rebuild_point_lights = true;
                }
                Event::CreateSpotLight(_, _) | Event::DestroySpotLight(_) => {
                    rebuild_spot_lights = true;
                }
            }
        }

        if rebuild_dir_lights {
            self.directional_lights_buffer =
                update_directional_lights(device, self.directional_lights.values().copied());
        }

        if rebuild_point_lights {
            self.point_lights_buffer =
                update_point_lights(device, self.point_lights.values().copied());
        }

        if rebuild_spot_lights {
            self.spot_lights_buffer =
                update_spot_lights(device, self.spot_lights.values().copied());
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum Event {
    CreateCamera(CameraId, Camera),
    DestroyCamera(CameraId),
    CreateObject(ObjectId, Object),
    DestroyObject(ObjectId),
    CreateDirectionalLight(DirectionalLightId, DirectionalLight),
    DestroyDirectionalLight(DirectionalLightId),
    CreatePointLight(PointLightId, PointLight),
    DestroyPointLight(PointLightId),
    CreateSpotLight(SpotLightId, SpotLight),
    DestroySpotLight(SpotLightId),
}

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
pub struct MaterialUniform {
    pub color: [f32; 4],
    pub metallic: f32,
    pub roughness: f32,
    pub reflectance: f32,
    pub _pad0: [u32; 1],
    pub albedo_texture: u32,
    pub normal_texture: u32,
    pub metallic_roughness_texture: u32,
    pub _pad1: [u32; 1],
}

impl GpuBuffer for MaterialUniform {
    const SIZE: usize = core::mem::size_of::<Self>();
    const ALIGN: usize = 16;
}
