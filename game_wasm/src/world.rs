use core::mem::MaybeUninit;

use alloc::vec::Vec;

use crate::components::Component;
use crate::encoding::{decode_fields, encode_value, BinaryReader};
use crate::entity::EntityId;
use crate::player::PlayerId;
use crate::raw::world::{
    world_entity_component_get, world_entity_component_insert, world_entity_component_len,
    world_entity_component_remove, world_entity_despawn, world_entity_spawn,
};
use crate::raw::{RESULT_NO_COMPONENT, RESULT_NO_ENTITY, RESULT_OK};
pub use crate::record::RecordReference;
use crate::{unreachable_unchecked, Error, ErrorImpl};

#[derive(Clone)]
pub struct Entity(EntityId);

impl Entity {
    pub fn new(id: EntityId) -> Self {
        Self(id)
    }

    pub fn spawn() -> Self {
        let mut entity_id = MaybeUninit::uninit();
        match unsafe { world_entity_spawn(entity_id.as_mut_ptr()) } {
            RESULT_OK => (),
            _ => unsafe { unreachable_unchecked() },
        }

        Self(EntityId::from_raw(unsafe { entity_id.assume_init() }))
    }

    pub fn get<T>(&self) -> Result<T, Error>
    where
        T: Component,
    {
        let entity_id = self.0.into_raw();
        let component_id = T::ID;

        let mut data_len = 0;
        let mut fields_len = 0;
        match unsafe {
            world_entity_component_len(entity_id, &component_id, &mut data_len, &mut fields_len)
        } {
            RESULT_OK => (),
            RESULT_NO_ENTITY => return Err(Error(ErrorImpl::NoEntity(self.0))),
            RESULT_NO_COMPONENT => return Err(Error(ErrorImpl::NoComponent(T::ID))),
            _ => unsafe { unreachable_unchecked() },
        }

        let mut data = Vec::with_capacity(data_len);
        let mut fields = Vec::with_capacity(fields_len);

        match unsafe {
            world_entity_component_get(
                entity_id,
                &component_id,
                data.as_mut_ptr(),
                fields.as_mut_ptr(),
            )
        } {
            RESULT_OK => (),
            // If our previous call to `world_entity_component_len` suceeds and does
            // not return `RESULT_NO_ENTITY` or `RESULT_NO_COMPONENT` this call will
            // also not return these results.
            RESULT_NO_ENTITY => unsafe { unreachable_unchecked() },
            RESULT_NO_COMPONENT => unsafe { unreachable_unchecked() },
            _ => unsafe { unreachable_unchecked() },
        }

        unsafe {
            data.set_len(data_len);
            fields.set_len(fields_len);
        }

        let fields = decode_fields(&fields);
        let reader = BinaryReader::new(data, fields.into());

        match T::decode(reader) {
            Ok(component) => Ok(component),
            Err(_) => Err(Error(ErrorImpl::ComponentDecode)),
        }
    }

    pub fn insert<T>(&self, component: T)
    where
        T: Component,
    {
        let (data, fields) = encode_value(&component);

        let entity_id = self.0.into_raw();
        let component_id = T::ID;

        match unsafe {
            world_entity_component_insert(
                entity_id,
                &component_id,
                data.as_ptr(),
                data.len(),
                fields.as_ptr(),
                fields.len(),
            )
        } {
            RESULT_OK => {}
            RESULT_NO_ENTITY => {
                panic!("no entity: {:?}", self.0);
            }
            _ => unsafe { unreachable_unchecked() },
        }
    }

    pub fn remove<T>(&self)
    where
        T: Component,
    {
        let entity_id = self.0.into_raw();
        let component_id = T::ID;

        match unsafe { world_entity_component_remove(entity_id, &component_id) } {
            RESULT_OK => {}
            RESULT_NO_ENTITY => {
                panic!("no entity: {:?}", self.0);
            }
            RESULT_NO_COMPONENT => {
                panic!("no component: {:?}", component_id);
            }
            _ => unsafe { unreachable_unchecked() },
        }
    }

    pub fn despawn(self) {
        let entity_id = self.0.into_raw();
        match unsafe { world_entity_despawn(entity_id) } {
            RESULT_OK => (),
            RESULT_NO_ENTITY => {
                panic!("no entity: {:?}", self.0);
            }
            _ => unsafe { unreachable_unchecked() },
        }
    }

    #[inline]
    pub fn id(&self) -> EntityId {
        self.0
    }

    pub fn player(&self) -> Option<PlayerId> {
        crate::player::player_lookup_safe(self.0)
    }

    /// Swaps the same component of two entities if both entities have the component.
    pub fn swap<T>(&self, other: Entity)
    where
        T: Component,
    {
        let (Ok(lhs), Ok(rhs)) = (self.get::<T>(), other.get::<T>()) else {
            return;
        };

        self.insert(rhs);
        other.insert(lhs);
    }

    /// Copies a component from another entity.
    pub fn copy_from<T>(&self, other: Entity)
    where
        T: Component,
    {
        let Ok(component) = other.get::<T>() else {
            return;
        };

        self.insert(component);
    }
}

// #[derive(Debug)]
// pub enum ComponentEntry<'a> {
//     Occupied(OccupiedComponentEntry<'a>),
//     Vacant(VacantComponentEntry<'a>),
// }

// #[derive(Debug)]
// pub struct OccupiedComponentEntry<'a> {
//     components: &'a EntityComponents,
//     id: RecordReference,
//     component: Component,
// }

// impl<'a> OccupiedComponentEntry<'a> {
//     #[inline]
//     pub fn key(&self) -> RecordReference {
//         self.id
//     }

//     pub fn get(&self) -> &Component {
//         &self.component
//     }

//     pub fn get_mut(&mut self) -> &mut Component {
//         &mut self.component
//     }

//     pub fn remove(self) -> Component {
//         self.components.remove(self.id).unwrap();
//         self.component
//     }
// }

// #[derive(Debug)]
// pub struct VacantComponentEntry<'a> {
//     components: &'a EntityComponents,
//     id: RecordReference,
// }

// impl<'a> VacantComponentEntry<'a> {
//     pub fn insert(self, value: Component) -> Component {
//         self.components.insert(self.id, &value).unwrap();
//         value
//     }

//     #[inline]
//     pub fn key(&self) -> RecordReference {
//         self.id
//     }
// }

// #[derive(Clone)]
// pub struct EntityBuilder {
//     translation: Vec3,
//     rotation: Quat,
//     scale: Vec3,
//     kind: RawEntityKind,
//     body: EntityBody,
//     components: Components,
//     linvel: Vec3,
//     angvel: Vec3,
// }

// impl EntityBuilder {
//     pub fn from_record(id: RecordReference) -> Self {
//         let record = Record::get(id);

//         let (kind, body) = match record.kind {
//             RecordKind::Item => (RawEntityKind::ITEM, EntityBody { item: id }),
//             RecordKind::Object => (RawEntityKind::OBJECT, EntityBody { object: id }),
//             RecordKind::Race => (RawEntityKind::ACTOR, EntityBody { actor: [0u8; 20] }),
//         };

//         Self {
//             translation: Vec3::ZERO,
//             rotation: Quat::IDENTITY,
//             scale: Vec3::splat(1.0),
//             components: record.components,
//             kind,
//             body,
//             linvel: Vec3::ZERO,
//             angvel: Vec3::ZERO,
//         }
//     }

//     pub fn new<T>(entity: T) -> Self
//     where
//         T: IntoEntityBody,
//     {
//         Self {
//             translation: Vec3::ZERO,
//             rotation: Quat::IDENTITY,
//             scale: Vec3::splat(1.0),
//             kind: entity.kind(),
//             body: entity.body(),
//             components: Components::new(),
//             linvel: Vec3::ZERO,
//             angvel: Vec3::ZERO,
//         }
//     }

//     pub fn translation(mut self, translation: Vec3) -> Self {
//         self.translation = translation;
//         self
//     }

//     pub fn rotation(mut self, rotation: Quat) -> Self {
//         self.rotation = rotation;
//         self
//     }

//     pub fn scale(mut self, scale: Vec3) -> Self {
//         self.scale = scale;
//         self
//     }

//     pub fn component(mut self, id: RecordReference, component: Component) -> Self {
//         self.components.insert(id, component);
//         self
//     }

//     pub fn linvel(mut self, linvel: Vec3) -> Self {
//         self.linvel = linvel;
//         self
//     }

//     pub fn angvel(mut self, angvel: Vec3) -> Self {
//         self.angvel = angvel;
//         self
//     }

//     /// Spawns this entity.
//     pub fn spawn(&self) -> Result<EntityId, Error> {
//         let mut entity_id = MaybeUninit::uninit();

//         let entity = raw::Entity {
//             id: 0,
//             translation: self.translation.to_array(),
//             rotation: self.rotation.to_array(),
//             scale: self.scale.to_array(),
//             kind: self.kind,
//             body: self.body,
//             linvel: self.linvel.to_array(),
//             angvel: self.angvel.to_array(),
//         };

//         let res = unsafe {
//             raw::world_entity_spawn(
//                 Ptr::from_ptr(&entity),
//                 PtrMut::from_ptr(entity_id.as_mut_ptr()),
//             )
//         };
//         assert!(res == RESULT_OK);

//         let entity_id = unsafe { entity_id.assume_init() };

//         for (id, component) in &self.components {
//             let res = unsafe {
//                 raw::world_entity_component_insert(
//                     entity_id,
//                     Ptr::from_ptr(&id),
//                     Ptr::from_ptr(component.as_ptr()),
//                     component.len() as u32,
//                 )
//             };
//             debug_assert!(res == RESULT_OK);
//         }

//         Ok(EntityId::from_raw(entity_id))
//     }
// }

// #[derive(Copy, Clone, Debug)]
// pub struct Object {
//     pub id: RecordReference,
// }

// #[derive(Copy, Clone, Debug)]
// pub struct Item {
//     pub id: RecordReference,
// }

// pub unsafe trait IntoEntityBody: private::Sealed {
//     #[doc(hidden)]
//     fn kind(&self) -> RawEntityKind;

//     #[doc(hidden)]
//     fn body(&self) -> EntityBody;
// }

// unsafe impl IntoEntityBody for Object {
//     fn kind(&self) -> RawEntityKind {
//         RawEntityKind::OBJECT
//     }

//     fn body(&self) -> EntityBody {
//         EntityBody { object: self.id }
//     }
// }

// impl private::Sealed for Object {}

// unsafe impl IntoEntityBody for Item {
//     fn kind(&self) -> RawEntityKind {
//         RawEntityKind::ITEM
//     }

//     fn body(&self) -> EntityBody {
//         EntityBody { item: self.id }
//     }
// }

// impl private::Sealed for Item {}

// mod private {
//     pub trait Sealed {}
// }

// #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
// pub enum EntityKind {
//     Terrain,
//     Object,
//     Actor,
//     Item,
// }

// impl EntityKind {
//     #[inline]
//     pub const fn is_terrain(self) -> bool {
//         matches!(self, Self::Terrain)
//     }

//     #[inline]
//     pub const fn is_object(self) -> bool {
//         matches!(self, Self::Object)
//     }

//     #[inline]
//     pub const fn is_actor(self) -> bool {
//         matches!(self, Self::Actor)
//     }

//     #[inline]
//     pub const fn is_item(self) -> bool {
//         matches!(self, Self::Item)
//     }
// }
