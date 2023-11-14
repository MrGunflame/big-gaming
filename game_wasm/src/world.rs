use core::mem::MaybeUninit;

use alloc::vec::Vec;
use glam::{Quat, Vec3};

use crate::component::{Component, Components};
use crate::entity::EntityId;
use crate::raw::{Ptr, PtrMut, Usize, ERROR_NO_ENTITY};
pub use crate::record::RecordReference;
use crate::record::{Record, RecordKind};
use crate::Error;

use crate::raw::world::{self as raw, EntityBody, EntityKind as RawEntityKind};

/// The requested entity does not exist.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct NoEntity {
    _priv: (),
}

#[derive(Clone)]
pub struct Entity(raw::Entity);

impl Entity {
    /// Returns the `Entity` with the given `id`.
    ///
    /// # Errors
    ///
    /// Returns [`NoEntity`] if the requested `id` does not currently exist.
    pub fn get(id: EntityId) -> Result<Self, NoEntity> {
        let mut entity = MaybeUninit::<raw::Entity>::uninit();
        let ptr = entity.as_mut_ptr() as Usize;

        let res = unsafe { raw::world_entity_get(id.into_raw(), PtrMut::from_raw(ptr)) };

        if res == 0 {
            Ok(Self(unsafe { entity.assume_init_read() }))
        } else {
            debug_assert_eq!(res, ERROR_NO_ENTITY);

            Err(NoEntity { _priv: () })
        }
    }

    /// Spawns the entity.
    pub fn spawn(&mut self) -> Result<(), Error> {
        let mut id = self.0.id;
        let out_ptr = &mut id as *mut u64 as Usize;

        let ptr = &self.0 as *const raw::Entity as Usize;

        let res = unsafe { raw::world_entity_spawn(Ptr::from_raw(ptr), PtrMut::from_raw(out_ptr)) };

        if res == 0 {
            self.0.id = id;

            Ok(())
        } else {
            Err(Error)
        }
    }

    /// Despawns the `Entity`.
    ///
    /// # Errors
    ///
    /// Returns [`NoEntity`] if the requested `id` does not currently exist.
    pub fn despawn(&self) -> Result<(), NoEntity> {
        let res = unsafe { raw::world_entity_despawn(self.0.id) };

        if res == 0 {
            Ok(())
        } else {
            debug_assert_eq!(res, ERROR_NO_ENTITY);

            Err(NoEntity { _priv: () })
        }
    }

    pub fn components(&self) -> EntityComponents {
        EntityComponents { entity: self.0.id }
    }

    pub fn translation(&self) -> Vec3 {
        Vec3::from_array(self.0.translation)
    }

    pub fn set_translation(&mut self, translation: Vec3) {
        self.0.translation = translation.to_array();

        let [x, y, z] = translation.to_array();
        unsafe {
            let _ = raw::world_entity_set_translation(self.0.id, x, y, z);
        }
    }

    pub fn rotation(&self) -> Quat {
        Quat::from_array(self.0.rotation)
    }

    pub fn set_rotation(&mut self, rotation: Quat) {
        assert!(rotation.is_normalized());
        self.0.rotation = rotation.to_array();

        let [x, y, z, w] = rotation.to_array();
        unsafe {
            let _ = raw::world_entity_set_rotation(self.0.id, x, y, z, w);
        }
    }

    pub fn scale(&self) -> Vec3 {
        Vec3::from_array(self.0.scale)
    }

    pub fn kind(&self) -> EntityKind {
        match self.0.kind {
            RawEntityKind::TERRAIN => EntityKind::Terrain,
            RawEntityKind::OBJECT => EntityKind::Object,
            RawEntityKind::ACTOR => EntityKind::Actor,
            RawEntityKind::ITEM => EntityKind::Item,
            _ => unreachable!(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct EntityComponents {
    entity: u64,
}

impl EntityComponents {
    pub fn get(&self, id: RecordReference) -> Result<Component, Error> {
        let mut len: Usize = 0;
        let len_ptr = &mut len as *mut Usize as Usize;

        let res = unsafe {
            raw::world_entity_component_len(
                self.entity,
                Ptr::from_raw(&id as *const _ as Usize),
                PtrMut::from_raw(len_ptr),
            )
        };

        if res != 0 {
            return Err(Error);
        }

        // No need to fetch the component data when we know
        // that it is empty.
        if len == 0 {
            return Ok(Component::new(Vec::new()));
        }

        let mut bytes = Vec::with_capacity(len as usize);

        let res = unsafe {
            raw::world_entity_component_get(
                self.entity,
                Ptr::from_raw(&id as *const _ as Usize),
                PtrMut::from_raw(bytes.as_mut_ptr() as Usize),
                len,
            )
        };

        if res == 0 {
            unsafe {
                bytes.set_len(len as usize);
            }

            Ok(Component::new(bytes))
        } else {
            Err(Error)
        }
    }

    pub fn insert(&self, id: RecordReference, component: &Component) -> Result<(), Error> {
        let ptr = Ptr::from_raw(component.as_bytes().as_ptr() as Usize);
        let len = component.as_bytes().len() as Usize;

        let res = unsafe {
            raw::world_entity_component_insert(
                self.entity,
                Ptr::from_raw(&id as *const _ as Usize),
                ptr,
                len,
            )
        };

        if res == 0 {
            Ok(())
        } else {
            Err(Error)
        }
    }

    pub fn remove(&self, id: RecordReference) -> Result<(), Error> {
        let id = &id as *const _ as Usize;

        let res = unsafe { raw::world_entity_component_remove(self.entity, Ptr::from_raw(id)) };

        if res == 0 {
            Ok(())
        } else {
            Err(Error)
        }
    }
}

#[derive(Clone)]
pub struct EntityBuilder {
    translation: Vec3,
    rotation: Quat,
    scale: Vec3,
    kind: RawEntityKind,
    body: EntityBody,
    components: Components,
}

impl EntityBuilder {
    pub fn from_record(id: RecordReference) -> Self {
        let record = Record::get(id);

        let (kind, body) = match record.kind {
            RecordKind::Item => (RawEntityKind::ITEM, EntityBody { item: id }),
            RecordKind::Object => (RawEntityKind::OBJECT, EntityBody { object: id }),
            RecordKind::Race => (RawEntityKind::ACTOR, EntityBody { actor: [0u8; 20] }),
        };

        Self {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::splat(1.0),
            components: record.components,
            kind,
            body,
        }
    }

    pub fn new<T>(entity: T) -> Self
    where
        T: IntoEntityBody,
    {
        Self {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::splat(1.0),
            kind: entity.kind(),
            body: entity.body(),
            components: Components::new(),
        }
    }

    pub fn translation(mut self, translation: Vec3) -> Self {
        self.translation = translation;
        self
    }

    pub fn rotation(mut self, rotation: Quat) -> Self {
        self.rotation = rotation;
        self
    }

    pub fn scale(mut self, scale: Vec3) -> Self {
        self.scale = scale;
        self
    }

    /// Spawns this entity.
    pub fn spawn(&self) -> Result<EntityId, Error> {
        let mut entity_id = MaybeUninit::uninit();

        let entity = raw::Entity {
            id: 0,
            translation: self.translation.to_array(),
            rotation: self.rotation.to_array(),
            scale: self.scale.to_array(),
            kind: self.kind,
            body: self.body,
        };

        let res = unsafe {
            raw::world_entity_spawn(
                Ptr::from_ptr(&entity),
                PtrMut::from_ptr(entity_id.as_mut_ptr()),
            )
        };
        if res != 0 {
            return Err(Error);
        }

        let entity_id = unsafe { entity_id.assume_init() };

        for (id, component) in &self.components {
            let res = unsafe {
                raw::world_entity_component_insert(
                    entity_id,
                    Ptr::from_ptr(&id),
                    Ptr::from_ptr(component.as_ptr()),
                    component.len() as u32,
                )
            };
            if res != 0 {
                return Err(Error);
            }
        }

        Ok(EntityId::from_raw(entity_id))
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Object {
    pub id: RecordReference,
}

#[derive(Copy, Clone, Debug)]
pub struct Item {
    pub id: RecordReference,
}

pub unsafe trait IntoEntityBody: private::Sealed {
    #[doc(hidden)]
    fn kind(&self) -> RawEntityKind;

    #[doc(hidden)]
    fn body(&self) -> EntityBody;
}

unsafe impl IntoEntityBody for Object {
    fn kind(&self) -> RawEntityKind {
        RawEntityKind::OBJECT
    }

    fn body(&self) -> EntityBody {
        EntityBody { object: self.id }
    }
}

impl private::Sealed for Object {}

unsafe impl IntoEntityBody for Item {
    fn kind(&self) -> RawEntityKind {
        RawEntityKind::ITEM
    }

    fn body(&self) -> EntityBody {
        EntityBody { item: self.id }
    }
}

impl private::Sealed for Item {}

mod private {
    pub trait Sealed {}
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum EntityKind {
    Terrain,
    Object,
    Actor,
    Item,
}

impl EntityKind {
    #[inline]
    pub const fn is_terrain(self) -> bool {
        matches!(self, Self::Terrain)
    }

    #[inline]
    pub const fn is_object(self) -> bool {
        matches!(self, Self::Object)
    }

    #[inline]
    pub const fn is_actor(self) -> bool {
        matches!(self, Self::Actor)
    }

    #[inline]
    pub const fn is_item(self) -> bool {
        matches!(self, Self::Item)
    }
}
