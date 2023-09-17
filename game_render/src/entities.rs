use game_asset::{Assets, Handle};
use game_common::components::transform::Transform;
use slotmap::{DefaultKey, SlotMap};
use wgpu::{Device, Queue};

use crate::camera::{Camera, CameraBuffer};
use crate::forward::ForwardPipeline;
use crate::light::pipeline::{update_directional_lights, update_point_lights, update_spot_lights};
use crate::light::{DirectionalLight, PointLight, SpotLight};
use crate::mesh::Mesh;
use crate::mipmap::MipMapGenerator;
use crate::pbr::PbrMaterial;
use crate::render_pass::{GpuObject, GpuState};
use crate::texture::Images;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ObjectId(DefaultKey);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct DirectionalLightId(DefaultKey);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct PointLightId(DefaultKey);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct SpotLightId(DefaultKey);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct CameraId(DefaultKey);

#[derive(Debug)]
pub struct ObjectManager {
    objects: SlotMap<DefaultKey, Object>,
}

impl ObjectManager {
    pub fn new() -> Self {
        Self {
            objects: SlotMap::new(),
        }
    }

    pub fn insert(&mut self, object: Object) -> ObjectId {
        let key = self.objects.insert(object);
        ObjectId(key)
    }

    pub fn get(&self, id: ObjectId) -> Option<&Object> {
        self.objects.get(id.0)
    }

    pub fn get_mut(&mut self, id: ObjectId) -> Option<&mut Object> {
        self.objects.get_mut(id.0)
    }

    pub fn remove(&mut self, id: ObjectId) {
        self.objects.remove(id.0);
    }
}

impl Default for ObjectManager {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct DirectionalLightManager {
    lights: SlotMap<DefaultKey, DirectionalLight>,
}

impl DirectionalLightManager {
    pub fn new() -> Self {
        Self {
            lights: SlotMap::new(),
        }
    }

    pub fn insert(&mut self, light: DirectionalLight) -> DirectionalLightId {
        let key = self.lights.insert(light);
        DirectionalLightId(key)
    }

    pub fn remove(&mut self, id: DirectionalLightId) {
        self.lights.remove(id.0);
    }
}

impl Default for DirectionalLightManager {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct PointLightManager {
    lights: SlotMap<DefaultKey, PointLight>,
}

impl PointLightManager {
    pub fn new() -> Self {
        Self {
            lights: SlotMap::new(),
        }
    }

    pub fn insert(&mut self, light: PointLight) -> PointLightId {
        let key = self.lights.insert(light);
        PointLightId(key)
    }

    pub fn remove(&mut self, id: PointLightId) {
        self.lights.remove(id.0);
    }
}

impl Default for PointLightManager {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct SpotLightManager {
    lights: SlotMap<DefaultKey, SpotLight>,
}

impl SpotLightManager {
    pub fn new() -> Self {
        Self {
            lights: SlotMap::new(),
        }
    }

    pub fn insert(&mut self, light: SpotLight) -> SpotLightId {
        let key = self.lights.insert(light);
        SpotLightId(key)
    }

    pub fn remove(&mut self, id: SpotLightId) {
        self.lights.remove(id.0);
    }
}

impl Default for SpotLightManager {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct CameraManager {
    cameras: SlotMap<DefaultKey, Camera>,
}

impl CameraManager {
    pub fn new() -> Self {
        Self {
            cameras: SlotMap::new(),
        }
    }

    pub fn insert(&mut self, camera: Camera) -> CameraId {
        let key = self.cameras.insert(camera);
        CameraId(key)
    }

    pub fn get_mut(&mut self, id: CameraId) -> Option<&mut Camera> {
        self.cameras.get_mut(id.0)
    }

    pub fn remove(&mut self, id: CameraId) {
        self.cameras.remove(id.0);
    }

    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut Camera> {
        self.cameras.values_mut()
    }
}

impl Default for CameraManager {
    fn default() -> Self {
        Self::new()
    }
}

pub struct SceneEntities {
    objects: ObjectManager,
    cameras: CameraManager,
    directional_lights: DirectionalLightManager,
    point_lights: PointLightManager,
    spot_lights: SpotLightManager,
    update_objects: bool,
    update_cameras: bool,
    update_directional_lights: bool,
    update_point_lights: bool,
    update_spot_lights: bool,
    pub(crate) state: GpuState,
}

impl SceneEntities {
    pub fn new(device: &Device) -> Self {
        Self {
            objects: ObjectManager::new(),
            cameras: CameraManager::new(),
            directional_lights: DirectionalLightManager::new(),
            point_lights: PointLightManager::new(),
            spot_lights: SpotLightManager::new(),
            state: GpuState::new(device),
            update_cameras: false,
            update_directional_lights: false,
            update_objects: false,
            update_point_lights: false,
            update_spot_lights: false,
        }
    }

    pub fn objects(&mut self) -> &mut ObjectManager {
        self.update_objects = true;
        &mut self.objects
    }

    pub fn cameras(&mut self) -> &mut CameraManager {
        self.update_cameras = true;
        &mut self.cameras
    }

    pub fn directional_lights(&mut self) -> &mut DirectionalLightManager {
        self.update_directional_lights = true;
        &mut self.directional_lights
    }

    pub fn point_lights(&mut self) -> &mut PointLightManager {
        self.update_point_lights = true;
        &mut self.point_lights
    }

    pub fn spot_lights(&mut self) -> &mut SpotLightManager {
        self.update_spot_lights = true;
        &mut self.spot_lights
    }
}

impl SceneEntities {
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
        if self.update_directional_lights {
            self.state.directional_lights =
                update_directional_lights(device, self.directional_lights.lights.values().copied());

            self.update_directional_lights = false;
        }

        if self.update_point_lights {
            self.state.point_lights =
                update_point_lights(device, self.point_lights.lights.values().copied());

            self.update_point_lights = false;
        }

        if self.update_spot_lights {
            self.state.spot_lights =
                update_spot_lights(device, self.spot_lights.lights.values().copied());

            self.update_spot_lights = false;
        }

        if self.update_cameras {
            self.state.cameras.clear();
            for (id, cam) in &mut self.cameras.cameras {
                let buffer =
                    CameraBuffer::new(cam.transform, cam.projection, device, cam.target.clone());

                self.state.cameras.insert(CameraId(id), buffer);
            }

            self.update_cameras = false;
        }

        if self.update_objects {
            self.state.objects.clear();
            for (id, obj) in &mut self.objects.objects {
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
                    ObjectId(id),
                    GpuObject {
                        material_bind_group: mat_bg,
                        mesh_bind_group: mesh_bg,
                        transform,
                        indices,
                    },
                );
            }

            self.update_objects = false;
        }
    }
}

#[derive(Debug)]
pub struct Object {
    pub transform: Transform,
    pub mesh: Handle<Mesh>,
    pub material: Handle<PbrMaterial>,
}
