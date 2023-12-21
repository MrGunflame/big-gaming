use alloc::vec::Vec;

use crate::components::AsComponent;
use crate::entity::EntityId;
use crate::raw::world::{
    world_entity_component_get, world_entity_component_insert, world_entity_component_len,
    world_entity_component_remove,
};
use crate::raw::{RESULT_NO_COMPONENT, RESULT_NO_ENTITY, RESULT_OK};
pub use crate::record::RecordReference;
use crate::unreachable_unchecked;

#[derive(Clone)]
pub struct Entity(EntityId);

impl Entity {
    pub fn new(id: EntityId) -> Self {
        Self(id)
    }

    pub fn get<T>(&self) -> T
    where
        T: AsComponent,
    {
        let entity_id = self.0.into_raw();
        let component_id = T::ID;

        let mut len = 0;
        match unsafe { world_entity_component_len(entity_id, &component_id, &mut len) } {
            RESULT_OK => (),
            RESULT_NO_ENTITY => {
                panic!("no entity: {:?}", self.0)
            }
            RESULT_NO_COMPONENT => {
                panic!("no component: {:?}", component_id)
            }
            _ => unsafe { unreachable_unchecked() },
        }

        let mut bytes = Vec::with_capacity(len as usize);
        match unsafe {
            world_entity_component_get(entity_id, &component_id, bytes.as_mut_ptr(), len)
        } {
            RESULT_OK => (),
            RESULT_NO_ENTITY => {
                panic!("no entity: {:?}", self.0);
            }
            RESULT_NO_COMPONENT => {
                panic!("no component: {:?}", component_id);
            }
            _ => unsafe { unreachable_unchecked() },
        }

        unsafe {
            bytes.set_len(len as usize);
        }

        T::from_bytes(&bytes)
    }

    pub fn insert<T>(&self, component: T)
    where
        T: AsComponent,
    {
        let entity_id = self.0.into_raw();
        let component_id = T::ID;
        let bytes = component.to_bytes();
        let len = bytes.len() as u32;

        match unsafe {
            world_entity_component_insert(entity_id, &component_id, bytes.as_ptr(), len)
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
        T: AsComponent,
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
