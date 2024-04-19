use std::alloc::Layout;
use std::collections::HashMap;

use game_tracing::trace_span;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{BindGroup, Buffer, BufferUsages, Device, Queue};

use crate::allocator::{Allocation, Allocator};
use crate::buffer::DynamicBuffer;
use crate::camera::{Camera, CameraBuffer};
use crate::entities::{CameraId, DirectionalLightId, Object, ObjectId, PointLightId, SpotLightId};
use crate::forward::{DrawData, ForwardPipeline, IndexData};
use crate::light::pipeline::{
    update_directional_lights, update_point_lights, update_spot_lights, DirectionalLightUniform,
    PointLightUniform, SpotLightUniform,
};
use crate::light::{DirectionalLight, PointLight, SpotLight};
use crate::mesh::Mesh;
use crate::mipmap::MipMapGenerator;
use crate::pbr::material::{update_material_bind_group, MaterialId, Materials};
use crate::pbr::mesh::{update_transform_buffer, MeshId, Meshes, TransformUniform};
use crate::pbr::PbrMaterial;
use crate::texture::{Image, ImageId, Images};

pub(crate) struct RenderState {
    pub cameras: HashMap<CameraId, Camera>,
    pub camera_buffers: HashMap<CameraId, CameraBuffer>,
    pub objects: HashMap<ObjectId, Object>,

    pub directional_lights: HashMap<DirectionalLightId, DirectionalLight>,
    pub point_lights: HashMap<PointLightId, PointLight>,
    pub spot_lights: HashMap<SpotLightId, SpotLight>,
    pub directional_lights_buffer: Buffer,
    pub point_lights_buffer: Buffer,
    pub spot_lights_buffer: Buffer,

    pub events: Vec<Event>,

    pub meshes: HashMap<MeshId, MeshData>,
    pub materials: HashMap<MaterialId, BindGroup>,

    pub meshes_queued: HashMap<MeshId, Mesh>,
    pub materials_queued: HashMap<MaterialId, PbrMaterial>,
    pub images: HashMap<ImageId, Image>,

    pub vertex_allocator: Allocator,
    pub index_allocator: Allocator,

    // TODO: Move completely to GPU device local memory.
    pub vertex_buffer: Vec<u8>,
    pub index_buffer: Vec<u8>,

    pub draw_data: HashMap<ObjectId, (DrawData, IndexData)>,
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
            camera_buffers: HashMap::new(),
            directional_lights: HashMap::new(),
            point_lights: HashMap::new(),
            spot_lights: HashMap::new(),
            vertex_allocator: Allocator::new(0),
            index_allocator: Allocator::new(0),
            vertex_buffer: Vec::new(),
            index_buffer: Vec::new(),
            draw_data: HashMap::new(),
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
            let num_positions = mesh.positions().len() * 3;
            let num_normals = mesh.normals().len() * 3;
            let num_uvs = mesh.uvs().len() * 2;
            let num_tangents = mesh.tangents().len() * 4;

            let required_size = (num_positions + num_normals + num_uvs + num_tangents)
                * core::mem::size_of::<f32>();

            if self.vertex_allocator.free_size() < required_size {
                self.vertex_buffer
                    .resize(self.vertex_allocator.max_size() + required_size, 0);

                self.vertex_allocator
                    .grow(self.vertex_allocator.max_size() + required_size);
            }

            let required_index_size =
                mesh.indicies().unwrap().as_u32().len() * core::mem::size_of::<u32>();
            if self.index_allocator.free_size() < required_index_size {
                self.index_buffer
                    .resize(self.index_allocator.max_size() + required_index_size, 0);

                self.index_allocator
                    .grow(self.index_allocator.max_size() + required_size);
            }

            let positions = self
                .vertex_allocator
                .alloc(Layout::array::<f32>(mesh.positions().len() * 3).unwrap())
                .unwrap();
            let normals = self
                .vertex_allocator
                .alloc(Layout::array::<f32>(mesh.normals().len() * 3).unwrap())
                .unwrap();
            let tangents = self
                .vertex_allocator
                .alloc(Layout::array::<f32>(mesh.tangents().len() * 4).unwrap())
                .unwrap();
            let uvs = self
                .vertex_allocator
                .alloc(Layout::array::<f32>(mesh.uvs().len() * 2).unwrap())
                .unwrap();

            self.vertex_buffer[positions.ptr()..positions.ptr() + positions.size()]
                .copy_from_slice(bytemuck::cast_slice(mesh.positions()));
            self.vertex_buffer[normals.ptr()..normals.ptr() + normals.size()]
                .copy_from_slice(bytemuck::cast_slice(mesh.normals()));
            self.vertex_buffer[tangents.ptr()..tangents.ptr() + tangents.size()]
                .copy_from_slice(bytemuck::cast_slice(mesh.tangents()));
            self.vertex_buffer[uvs.ptr()..uvs.ptr() + uvs.size()]
                .copy_from_slice(bytemuck::cast_slice(mesh.uvs()));

            let indices = self
                .index_allocator
                .alloc(Layout::array::<u32>(mesh.indicies().unwrap().as_u32().len()).unwrap())
                .unwrap();

            self.index_buffer[indices.ptr()..indices.ptr() + indices.size()]
                .copy_from_slice(bytemuck::cast_slice(mesh.indicies().unwrap().as_u32()));

            self.meshes.insert(
                id,
                MeshData {
                    positions,
                    normals,
                    tangents,
                    uvs,
                    indices,
                },
            );
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
                    let mesh = self.meshes.get(&object.mesh).unwrap();

                    self.draw_data.insert(
                        id,
                        (
                            DrawData {
                                transform: object.transform.into(),
                                vertex_index: mesh.positions.ptr() as u32,
                                normal_index: mesh.normals.ptr() as u32,
                                tangent_index: mesh.tangents.ptr() as u32,
                                uv_index: mesh.uvs.ptr() as u32,
                            },
                            IndexData {
                                index_offset: mesh.indices.ptr() as u32,
                                index_length: mesh.indices.size() as u32 / 4,
                            },
                        ),
                    );
                }
                Event::DestroyObject(id) => {
                    self.objects.remove(&id);
                    self.draw_data.remove(&id);
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

#[derive(Debug)]
struct MeshData {
    positions: Allocation,
    normals: Allocation,
    tangents: Allocation,
    uvs: Allocation,
    indices: Allocation,
}
