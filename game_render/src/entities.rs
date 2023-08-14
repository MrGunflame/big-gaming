use std::collections::HashMap;

use game_asset::{Assets, Handle};
use game_common::components::transform::Transform;
use wgpu::{Device, Queue};

use crate::camera::{Camera, CameraBuffer};
use crate::forward::ForwardPipeline;
use crate::light::{DirectionalLight, PointLight, SpotLight};
use crate::mesh::Mesh;
use crate::mipmap::MipMapGenerator;
use crate::pbr::PbrMaterial;
use crate::render_pass::{GpuObject, GpuState};
use crate::texture::Images;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Entity(u64);

pub struct Entities {
    next_id: u64,
    pub objects: HashMap<Entity, Object>,
    pub cameras: HashMap<Entity, Camera>,
    pub directional_lights: HashMap<Entity, DirectionalLight>,
    pub point_lights: HashMap<Entity, PointLight>,
    pub spot_lights: HashMap<Entity, SpotLight>,
    pub state: GpuState,
    need_rebuild: bool,
}

impl Entities {
    pub fn new(device: &Device) -> Self {
        Self {
            next_id: 0,
            objects: HashMap::new(),
            cameras: HashMap::new(),
            directional_lights: HashMap::new(),
            point_lights: HashMap::new(),
            spot_lights: HashMap::new(),
            state: GpuState::new(device),
            need_rebuild: false,
        }
    }

    pub fn push_object(&mut self, obj: Object) -> Entity {
        self.need_rebuild = true;

        let id = Entity(self.next_id);
        self.next_id += 1;

        self.objects.insert(id, obj);
        id
    }

    pub fn push_dir_light(&mut self, light: DirectionalLight) -> Entity {
        self.need_rebuild = true;

        let id = Entity(self.next_id);
        self.next_id += 1;

        self.directional_lights.insert(id, light);
        id
    }

    pub fn push_point_light(&mut self, light: PointLight) -> Entity {
        self.need_rebuild = true;

        let id = Entity(self.next_id);
        self.next_id += 1;

        self.point_lights.insert(id, light);
        id
    }

    pub fn push_spot_light(&mut self, light: SpotLight) -> Entity {
        self.need_rebuild = true;

        let id = Entity(self.next_id);
        self.next_id += 1;

        self.spot_lights.insert(id, light);
        id
    }

    pub fn push_camera(&mut self, camera: Camera) -> Entity {
        self.need_rebuild = true;

        let id = Entity(self.next_id);
        self.next_id += 1;

        self.cameras.insert(id, camera);
        id
    }

    pub fn rebuild(
        &mut self,
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<PbrMaterial>,
        images: &mut Images,
        device: &Device,
        queue: &Queue,
        pipeline: &ForwardPipeline,
        mipmap_generator: &mut MipMapGenerator,
    ) {
        self.state.directional_lights = crate::light::pipeline::update_directional_lights(
            device,
            self.directional_lights.values().copied(),
        );
        self.state.point_lights = crate::light::pipeline::update_point_lights(
            device,
            self.point_lights.values().copied(),
        );
        self.state.spot_lights =
            crate::light::pipeline::update_spot_lights(device, self.spot_lights.values().copied());

        self.state.cameras.clear();
        for (id, cam) in &mut self.cameras {
            let buffer =
                CameraBuffer::new(cam.transform, cam.projection, device, cam.target.clone());

            self.state.cameras.insert(*id, buffer);
        }

        self.state.objects.clear();
        for (id, obj) in &mut self.objects {
            let Some(mesh) = meshes.get(obj.mesh.id()) else {
                return;
            };

            let Some(material) = materials.get(obj.material.id()) else {
                return;
            };

            let (mesh_bg, indices) =
                crate::pbr::mesh::update_mesh_bind_group(device, pipeline, mesh);
            let mat_bg = crate::pbr::material::update_material_bind_group(
                device,
                queue,
                images,
                pipeline,
                material,
                mipmap_generator,
            );

            let transform = crate::pbr::mesh::update_transform_buffer(obj.transform, device);

            self.state.objects.insert(
                *id,
                GpuObject {
                    material_bind_group: mat_bg,
                    mesh_bind_group: mesh_bg,
                    transform,
                    indices,
                },
            );
        }

        self.need_rebuild = false;
    }
}

pub struct Object {
    pub transform: Transform,
    pub mesh: Handle<Mesh>,
    pub material: Handle<PbrMaterial>,
}
