use game_common::components::transform::Transform;
use slotmap::{new_key_type, Key, SlotMap};
use wgpu::Device;

use crate::camera::Camera;
use crate::light::{DirectionalLight, PointLight, SpotLight};
use crate::pbr::material::MaterialId;
use crate::pbr::mesh::MeshId;

#[derive(Clone, Debug)]
pub struct EntityManager<K: Key, V> {
    entities: SlotMap<K, V>,
}

impl<K: Key, V> EntityManager<K, V> {
    fn new() -> Self {
        Self {
            entities: SlotMap::default(),
        }
    }

    pub fn insert(&mut self, entity: V) -> K {
        self.entities.insert(entity)
    }

    pub fn get(&self, id: K) -> Option<&V> {
        self.entities.get(id)
    }

    pub fn get_mut(&mut self, id: K) -> Option<&mut V> {
        self.entities.get_mut(id)
    }

    pub fn remove(&mut self, id: K) {
        self.entities.remove(id);
    }

    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut V> {
        self.entities.values_mut()
    }

    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.entities.values()
    }
}

new_key_type! {
    pub struct ObjectId;
    pub struct DirectionalLightId;
    pub struct PointLightId;
    pub struct SpotLightId;
    pub struct CameraId;
}

#[derive(Clone, Debug)]
pub struct SceneEntities {
    pub objects: EntityManager<ObjectId, Object>,
    pub cameras: EntityManager<CameraId, Camera>,
    pub directional_lights: EntityManager<DirectionalLightId, DirectionalLight>,
    pub point_lights: EntityManager<PointLightId, PointLight>,
    pub spot_lights: EntityManager<SpotLightId, SpotLight>,
}

impl SceneEntities {
    pub fn new() -> Self {
        Self {
            objects: EntityManager::new(),
            cameras: EntityManager::new(),
            directional_lights: EntityManager::new(),
            point_lights: EntityManager::new(),
            spot_lights: EntityManager::new(),
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Object {
    pub transform: Transform,
    pub mesh: MeshId,
    pub material: MaterialId,
}
