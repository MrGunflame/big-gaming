use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::mem::ManuallyDrop;
use std::ops::{Deref, DerefMut};

use game_common::components::Transform;
use slotmap::{new_key_type, Key, SlotMap};

use crate::camera::Camera;
use crate::light::{DirectionalLight, PointLight, SpotLight};
use crate::pbr::material::MaterialId;
use crate::pbr::mesh::MeshId;
use crate::state::Event;

#[derive(Clone, Debug)]
pub struct EntityManager<K: Key, V: WithEvent<K>> {
    entities: SlotMap<K, V>,
    // We only want to maintain the most recent event for every entity.
    // This has the effect that later events overwrite earlier ones.
    // This is important as the consumer of the renderer must only maintain
    // the state of the current entity (the entity at the time the renderer
    // is dispached) and the renderer MUST NEVER attempt to create an entity
    // that is not current.
    // FIXME: Since we control the keys and they are already linear we can
    // use a Vec instead.
    pub(crate) events: HashMap<K, Event>,
}

impl<K: Key, V: WithEvent<K> + Copy> EntityManager<K, V> {
    fn new() -> Self {
        Self {
            entities: SlotMap::default(),
            events: HashMap::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.entities.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn insert(&mut self, entity: V) -> K {
        let id = self.entities.insert(entity);
        self.events.insert(id, V::create(id, entity));
        tracing::trace!("spawn entity {:?}", id);
        id
    }

    pub fn get(&self, id: K) -> Option<&V> {
        self.entities.get(id)
    }

    pub fn get_mut(&mut self, id: K) -> Option<EntityMut<'_, K, V>> {
        let entity = self.entities.get_mut(id)?;

        let event = self.events.entry(id);
        Some(EntityMut {
            id,
            entity,
            event: ManuallyDrop::new(event),
        })
    }

    pub fn remove(&mut self, id: K) {
        if self.entities.remove(id).is_some() {
            self.events.insert(id, V::destroy(id));
            tracing::trace!("despawn entity {:?}", id);
        }
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
            let event = self.events.entry(key);

            f(
                key,
                EntityMut {
                    id: key,
                    entity: val,
                    event: ManuallyDrop::new(event),
                },
            );
        }
    }

    pub(crate) fn drain_events(&mut self) -> impl Iterator<Item = Event> + '_ {
        self.events.drain().map(|(_, v)| v)
    }
}

impl<K: Key, V: WithEvent<K> + Copy> Default for EntityManager<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct EntityMut<'a, K, V>
where
    K: Key + Copy,
    V: WithEvent<K> + Copy,
{
    id: K,
    entity: &'a mut V,
    // Note that we use an `ManauallyDrop` here because we need to use the entry
    // API in the `Drop` impl, which requires an owned instance.
    event: ManuallyDrop<Entry<'a, K, Event>>,
}

impl<'a, K, V> Drop for EntityMut<'a, K, V>
where
    K: Key + Copy,
    V: WithEvent<K> + Copy,
{
    fn drop(&mut self) {
        let event = V::create(self.id, *self.entity);

        // SAFETY: We only take out the value once in the destructor.
        // See `EntityMut::event` why this is necessary here.
        let slot = unsafe { ManuallyDrop::take(&mut self.event) };
        match slot {
            Entry::Occupied(mut slot) => {
                slot.insert(event);
            }
            Entry::Vacant(slot) => {
                slot.insert(event);
            }
        }
    }
}

impl<'a, K, V> Deref for EntityMut<'a, K, V>
where
    K: Key + Copy,
    V: WithEvent<K> + Copy,
{
    type Target = V;

    fn deref(&self) -> &Self::Target {
        self.entity
    }
}

impl<'a, K, V> DerefMut for EntityMut<'a, K, V>
where
    K: Key + Copy,
    V: WithEvent<K> + Copy,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.entity
    }
}

new_key_type! {
    pub struct ObjectId;
    pub struct DirectionalLightId;
    pub struct PointLightId;
    pub struct SpotLightId;
    pub struct CameraId;
}

#[derive(Clone, Debug, Default)]
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
