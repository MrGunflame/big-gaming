use std::collections::HashMap;

use game_tracing::trace_span;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{BindGroup, BindGroupDescriptor, BindGroupEntry, Buffer, BufferUsages, Device, Queue};

use crate::buffer::{DynamicBuffer, IndexBuffer};
use crate::camera::Camera;
use crate::entities::{CameraId, DirectionalLightId, Object, ObjectId, PointLightId, SpotLightId};
use crate::forward::ForwardPipeline;
use crate::light::pipeline::{
    update_directional_lights, update_point_lights, update_spot_lights, DirectionalLightUniform,
    PointLightUniform, SpotLightUniform,
};
use crate::light::{DirectionalLight, PointLight, SpotLight};
use crate::mesh::Mesh;
use crate::mipmap::MipMapGenerator;
use crate::options::MainPassOptions;
use crate::pbr::material::{update_material_bind_group, MaterialId, Materials};
use crate::pbr::mesh::{update_mesh_bind_group, update_transform_buffer, MeshId, Meshes};
use crate::pbr::PbrMaterial;
use crate::texture::{Image, ImageId, Images};

pub(crate) struct RenderState {
    pub cameras: HashMap<CameraId, Camera>,
    pub objects: HashMap<ObjectId, Object>,
    /// object transform buffers
    pub object_buffers: HashMap<ObjectId, BindGroup>,

    pub directional_lights: HashMap<DirectionalLightId, DirectionalLight>,
    pub point_lights: HashMap<PointLightId, PointLight>,
    pub spot_lights: HashMap<SpotLightId, SpotLight>,
    pub directional_lights_buffer: Buffer,
    pub point_lights_buffer: Buffer,
    pub spot_lights_buffer: Buffer,

    pub events: Vec<Event>,

    pub meshes: HashMap<MeshId, (BindGroup, IndexBuffer)>,
    pub materials: HashMap<MaterialId, BindGroup>,

    pub meshes_queued: HashMap<MeshId, Mesh>,
    pub materials_queued: HashMap<MaterialId, PbrMaterial>,
    pub images: HashMap<ImageId, Image>,
    pub options: MainPassOptions,
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

        let mut imgs = HashMap::new();
        for id in [
            pipeline.default_textures.default_base_color_texture,
            pipeline.default_textures.default_normal_texture,
            pipeline.default_textures.default_metallic_roughness_texture,
        ] {
            let img = images.get(id).unwrap().clone();
            imgs.insert(id, img);
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
            images: imgs,
            meshes_queued: HashMap::new(),
            object_buffers: HashMap::new(),
            directional_lights: HashMap::new(),
            point_lights: HashMap::new(),
            spot_lights: HashMap::new(),
            options: MainPassOptions::default(),
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

        self.events.push(event.clone());

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
                        self.images.insert(id, img);
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
            Event::UpdateMainPassOptions(opts) => {
                self.options = opts;
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

        let mut rebuild_dir_lights = false;
        let mut rebuild_point_lights = false;
        let mut rebuild_spot_lights = false;

        for event in self.events.drain(..) {
            match event {
                Event::CreateCamera(_id, _camera) => (),
                Event::DestroyCamera(id) => {
                    self.cameras.remove(&id);
                }
                Event::CreateObject(id, object) => {
                    let buffer = update_transform_buffer(object.transform, device);
                    let bg = device.create_bind_group(&BindGroupDescriptor {
                        label: None,
                        layout: &pipeline.vs_bind_group_layout,
                        entries: &[BindGroupEntry {
                            binding: 0,
                            resource: buffer.as_entire_binding(),
                        }],
                    });

                    self.object_buffers.insert(id, bg);
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
                Event::UpdateMainPassOptions(_) => (),
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

#[derive(Clone, Debug)]
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
    UpdateMainPassOptions(MainPassOptions),
}
