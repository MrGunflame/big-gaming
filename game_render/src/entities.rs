use std::ops::{Deref, DerefMut};

use game_common::components::transform::Transform;
use slotmap::{new_key_type, Key, SlotMap};

use crate::camera::Camera;
use crate::light::{DirectionalLight, PointLight, SpotLight};
use crate::pbr::material::MaterialId;
use crate::pbr::mesh::MeshId;
use crate::state::Event;

#[derive(Clone, Debug)]
pub struct EntityManager<K: Key, V: WithEvent<K>> {
    entities: SlotMap<K, V>,
    pub(crate) events: Vec<Event>,
}

impl<K: Key, V: WithEvent<K> + Copy> EntityManager<K, V> {
    fn new() -> Self {
        Self {
            entities: SlotMap::default(),
            events: vec![],
        }
    }

    pub fn insert(&mut self, entity: V) -> K {
        let id = self.entities.insert(entity);
        self.events.push(V::create(id, entity));
        id
    }

    pub fn get(&self, id: K) -> Option<&V> {
        self.entities.get(id)
    }

    pub fn get_mut(&mut self, id: K) -> Option<EntityMut<'_, K, V>> {
        let entity = self.entities.get_mut(id)?;
        Some(EntityMut {
            id,
            entity,
            events: &mut self.events,
        })
    }

    pub fn remove(&mut self, id: K) {
        self.entities.remove(id);
        self.events.push(V::destroy(id));
    }

    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.entities.values()
    }

    // Provide this instead of `iter_mut` iterator, because it cannot be implemented
    // soundly with the current event system.
    pub(crate) fn for_each_mut<F>(&mut self, mut f: F)
    where
        F: FnMut(K, EntityMut<'_, K, V>),
    {
        for (key, val) in self.entities.iter_mut() {
            f(
                key,
                EntityMut {
                    id: key,
                    entity: val,
                    events: &mut self.events,
                },
            );
        }
    }
}

pub struct EntityMut<'a, K, V>
where
    K: Key + Copy,
    V: WithEvent<K> + Copy,
{
    id: K,
    entity: &'a mut V,
    events: &'a mut Vec<Event>,
}

impl<'a, K, V> Drop for EntityMut<'a, K, V>
where
    K: Key + Copy,
    V: WithEvent<K> + Copy,
{
    fn drop(&mut self) {
        self.events.push(V::create(self.id, *self.entity));
    }
}

impl<'a, K, V> Deref for EntityMut<'a, K, V>
where
    K: Key + Copy,
    V: WithEvent<K> + Copy,
{
    type Target = V;

    fn deref(&self) -> &Self::Target {
        &self.entity
    }
}

impl<'a, K, V> DerefMut for EntityMut<'a, K, V>
where
    K: Key + Copy,
    V: WithEvent<K> + Copy,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.entity
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

pub trait WithEvent<K> {
    fn create(id: K, v: Self) -> Event;
    fn destroy(id: K) -> Event;
}

impl WithEvent<ObjectId> for Object {
    fn create(id: ObjectId, v: Self) -> Event {
        Event::CreateObject(id, v)
    }

    fn destroy(id: ObjectId) -> Event {
        Event::DestroyObject(id)
    }
}

impl WithEvent<CameraId> for Camera {
    fn create(id: CameraId, v: Self) -> Event {
        Event::CreateCamera(id, v)
    }

    fn destroy(id: CameraId) -> Event {
        Event::DestroyCamera(id)
    }
}

impl WithEvent<DirectionalLightId> for DirectionalLight {
    fn create(id: DirectionalLightId, v: Self) -> Event {
        Event::CreateDirectionalLight(id, v)
    }

    fn destroy(id: DirectionalLightId) -> Event {
        Event::DestroyDirectionalLight(id)
    }
}

impl WithEvent<PointLightId> for PointLight {
    fn create(id: PointLightId, v: Self) -> Event {
        Event::CreatePointLight(id, v)
    }

    fn destroy(id: PointLightId) -> Event {
        Event::DestroyPointLight(id)
    }
}

impl WithEvent<SpotLightId> for SpotLight {
    fn create(id: SpotLightId, v: Self) -> Event {
        Event::CreateSpotLight(id, v)
    }

    fn destroy(id: SpotLightId) -> Event {
        Event::DestroySpotLight(id)
    }
}
