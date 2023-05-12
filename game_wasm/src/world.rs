use core::mem::{self, MaybeUninit};

use alloc::vec::Vec;
use bytemuck::AnyBitPattern;
use glam::{Quat, Vec3};

use crate::entity::EntityId;
use crate::raw::{Ptr, PtrMut, Usize};
pub use crate::record::RecordReference;
use crate::Error;

use crate::raw::world::{self as raw, EntityBody, EntityKind};

#[derive(Clone)]
pub struct Entity(raw::Entity);

impl Entity {
    pub fn get(id: EntityId) -> Result<Self, Error> {
        let mut entity = MaybeUninit::<raw::Entity>::uninit();
        let ptr = entity.as_mut_ptr() as Usize;

        let res = unsafe { raw::world_entity_get(id.into_raw(), PtrMut::from_raw(ptr)) };

        if res == 0 {
            Ok(Self(unsafe { entity.assume_init_read() }))
        } else {
            Err(Error)
        }
    }

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

    pub fn despawn(&self) -> Result<(), Error> {
        let res = unsafe { raw::world_entity_despawn(self.0.id) };

        if res == 0 {
            Ok(())
        } else {
            Err(Error)
        }
    }

    pub fn components(&self) -> EntityComponents {
        EntityComponents { entity: self.0.id }
    }

    pub fn translation(&self) -> Vec3 {
        Vec3::from_array(self.0.translation)
    }

    pub fn rotation(&self) -> Quat {
        Quat::from_array(self.0.rotation)
    }

    pub fn scale(&self) -> Vec3 {
        Vec3::from_array(self.0.scale)
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
            return Ok(Component { bytes: Vec::new() });
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

        unsafe {
            bytes.set_len(len as usize);
        }

        if res == 0 {
            Ok(Component { bytes })
        } else {
            Err(Error)
        }
    }

    pub fn insert(&self, id: RecordReference, component: &Component) -> Result<(), Error> {
        let ptr = Ptr::from_raw(component.bytes.as_ptr() as Usize);
        let len = component.bytes.len() as Usize;

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

/// A byte buffer containing component data.
///
/// Note that the buffer has the alignment of `u8`. If you read values from the buffer you must use
/// [`read_unaligned`].
///
/// [`read_unaligned`]: ptr::read_unaligned
#[derive(Clone, Debug, PartialEq)]
pub struct Component {
    bytes: Vec<u8>,
}

impl Component {
    /// Reads the value `T` from the buffer.
    ///
    /// # Panics
    ///
    /// Panics if the buffer is not big enough to hold `T`.
    #[inline]
    pub fn read<T>(&self) -> T
    where
        T: AnyBitPattern,
    {
        assert!(self.len() >= mem::size_of::<T>());

        // SAFETY: We have validated that the buffer is big enough for `T`.
        unsafe { self.read_unchecked() }
    }

    /// Reads the value `T` from the buffer without checking that the buffer is big enough.
    ///
    /// Note that the read is always unaligned and the buffer must not be correctly aligned for `T`.
    ///
    /// # Safety
    ///
    /// The buffer must have at least `mem::size_of::<T>` bytes.
    #[inline]
    pub unsafe fn read_unchecked<T>(&self) -> T
    where
        T: AnyBitPattern,
    {
        debug_assert!(self.bytes.len() >= mem::size_of::<T>());

        // SAFETY: `T` implements `AnyBitPattern`, which means any
        // read possible value is inhabitet.
        // The caller guarantees that `bytes.len() >= size_of::<T>()`.
        unsafe { (self.bytes.as_ptr() as *const T).read_unaligned() }
    }

    pub fn len(&self) -> usize {
        self.bytes.len()
    }
}

impl AsRef<[u8]> for Component {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        &self.bytes
    }
}

#[derive(Clone)]
pub struct EntityBuilder {
    translation: Vec3,
    rotation: Quat,
    scale: Vec3,
    kind: EntityKind,
    body: EntityBody,
}

impl EntityBuilder {
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

    pub fn build(self) -> Entity {
        Entity(raw::Entity {
            id: 0,
            translation: self.translation.to_array(),
            rotation: self.rotation.to_array(),
            scale: self.scale.to_array(),
            kind: self.kind,
            body: self.body,
        })
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
    fn kind(&self) -> EntityKind;

    #[doc(hidden)]
    fn body(&self) -> EntityBody;
}

unsafe impl IntoEntityBody for Object {
    fn kind(&self) -> EntityKind {
        EntityKind::OBJECT
    }

    fn body(&self) -> EntityBody {
        EntityBody { object: self.id }
    }
}

impl private::Sealed for Object {}

unsafe impl IntoEntityBody for Item {
    fn kind(&self) -> EntityKind {
        EntityKind::ITEM
    }

    fn body(&self) -> EntityBody {
        EntityBody { item: self.id }
    }
}

impl private::Sealed for Item {}

mod private {
    pub trait Sealed {}
}
