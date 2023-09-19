use std::collections::HashMap;

use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{BindGroup, Buffer, BufferUsages, Device, Queue};

use crate::buffer::{DynamicBuffer, IndexBuffer};
use crate::camera::{Camera, CameraBuffer};
use crate::entities::{CameraId, DirectionalLightId, Object, ObjectId, PointLightId, SpotLightId};
use crate::forward::ForwardPipeline;
use crate::light::pipeline::{DirectionalLightUniform, PointLightUniform, SpotLightUniform};
use crate::light::{DirectionalLight, PointLight, SpotLight};
use crate::mesh::Mesh;
use crate::mipmap::MipMapGenerator;
use crate::pbr::material::{update_material_bind_group, MaterialId, Materials};
use crate::pbr::mesh::{update_mesh_bind_group, update_transform_buffer, MeshId, Meshes};
use crate::pbr::PbrMaterial;
use crate::texture::{Image, ImageId, Images};

pub(crate) struct RenderState {
    pub cameras: HashMap<CameraId, Camera>,
    pub camera_buffers: HashMap<CameraId, CameraBuffer>,
    pub objects: HashMap<ObjectId, Object>,
    /// object transform buffers
    pub object_buffers: HashMap<ObjectId, Buffer>,
    pub directional_lights: Buffer,
    pub point_lights: Buffer,
    pub spot_lights: Buffer,

    pub events: Vec<Event>,

    pub meshes: HashMap<MeshId, (BindGroup, IndexBuffer)>,
    pub materials: HashMap<MaterialId, BindGroup>,

    pub meshes_queued: HashMap<MeshId, Mesh>,
    pub materials_queued: HashMap<MaterialId, PbrMaterial>,
    pub images: HashMap<ImageId, Image>,
}

impl RenderState {
    pub fn new(device: &Device) -> Self {
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
            directional_lights,
            spot_lights,
            point_lights,
            cameras: HashMap::new(),
            objects: HashMap::new(),
            events: vec![],
            meshes: HashMap::new(),
            materials: HashMap::new(),
            materials_queued: HashMap::new(),
            images: HashMap::new(),
            meshes_queued: HashMap::new(),
            camera_buffers: HashMap::new(),
            object_buffers: HashMap::new(),
        }
    }

    pub fn update(
        &mut self,
        event: Event,
        meshes: &Meshes,
        materials: &Materials,
        images: &Images,
    ) {
        self.events.push(event);

        match event {
            Event::CreateCamera(id, camera) => {
                self.cameras.insert(id, camera);
            }
            Event::DestroyCamera(id) => {}
            Event::CreateObject(id, object) => {
                self.objects.insert(id, object);

                if !self.meshes.contains_key(&object.mesh) {
                    let mesh = meshes.get(object.mesh).unwrap().clone();
                    self.meshes_queued.insert(object.mesh, mesh);
                }

                if !self.materials.contains_key(&object.material) {
                    let material = materials.get(object.material).unwrap().clone();
                    self.materials_queued.insert(object.material, material);

                    for tex in [
                        material.base_color_texture,
                        material.normal_texture,
                        material.metallic_roughness_texture,
                    ] {
                        if let Some(id) = tex {
                            let img = images.get(id).unwrap().clone();
                            self.images.insert(id, img);
                        }
                    }
                }
            }
            Event::DestroyObject(id) => {}
            _ => todo!(),
        }
    }

    pub fn update_buffers(
        &mut self,
        device: &Device,
        queue: &Queue,
        pipeline: &ForwardPipeline,
        mipmap_generator: &mut MipMapGenerator,
    ) {
        for (id, mesh) in self.meshes_queued.drain() {
            let (bg, buf) = update_mesh_bind_group(device, pipeline, &mesh);
            self.meshes.insert(id, (bg, buf));
        }

        for (id, material) in self.materials_queued.drain() {
            let bg = update_material_bind_group(
                device,
                queue,
                &self.images,
                pipeline,
                &material,
                mipmap_generator,
            );

            self.materials.insert(id, bg);
        }

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
                _ => todo!(),
            }
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
