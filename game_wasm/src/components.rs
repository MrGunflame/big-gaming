pub mod builtin;

pub use bytes::{Buf, BufMut};
pub use game_macros::wasm__component as Component;

use core::iter::FusedIterator;
use core::mem::{self};
use core::ptr::NonNull;

use alloc::vec::Vec;
use bytemuck::{AnyBitPattern, NoUninit, Pod};

use crate::encoding::{
    decode_fields, encode_value, BinaryReader, Decode, DecodeError, Encode, Field, Writer,
};
use crate::world::RecordReference;
use crate::{Error, ErrorImpl};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Components {
    // FIXME: We don't have access to HashMap in no_std.
    components: Vec<(RecordReference, RawComponent)>,
}

impl Components {
    pub fn new() -> Self {
        Self {
            components: Vec::new(),
        }
    }

    pub fn insert<T>(&mut self, component: T)
    where
        T: Component,
    {
        self.components.retain(|(id, _)| *id != T::ID);

        let (data, fields) = encode_value(&component);
        let fields = decode_fields(&fields);

        self.components.push((
            T::ID,
            RawComponent {
                bytes: data,
                fields,
            },
        ));
    }

    pub fn remove<T>(&mut self)
    where
        T: Component,
    {
        self.components.retain(|(id, _)| *id != T::ID);
    }

    pub fn get<T>(&self) -> Result<T, Error>
    where
        T: Component,
    {
        let (_, component) = self
            .components
            .iter()
            .find(|(id, _)| *id == T::ID)
            .ok_or(Error(ErrorImpl::NoComponent(T::ID)))?;

        let reader = BinaryReader::new(
            component.bytes.clone().into(),
            component.fields.clone().into(),
        );
        T::decode(reader).map_err(|_| Error(ErrorImpl::ComponentDecode))
    }

    pub fn get_mut(&mut self, id: RecordReference) -> Option<&mut RawComponent> {
        self.get_index(id)
            .map(|index| &mut self.components[index].1)
    }

    pub fn contains_key(&mut self, id: RecordReference) -> bool {
        self.components
            .iter()
            .any(|(component_id, _)| *component_id == id)
    }

    pub fn iter(&self) -> Iter<'_> {
        Iter {
            inner: self.components.iter(),
        }
    }

    fn get_index(&self, id: RecordReference) -> Option<usize> {
        for (index, (component_id, _)) in self.components.iter().enumerate() {
            if *component_id == id {
                return Some(index);
            }
        }

        None
    }
}

impl<'a> IntoIterator for &'a Components {
    type IntoIter = Iter<'a>;
    type Item = <Self::IntoIter as Iterator>::Item;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

pub struct Iter<'a> {
    inner: core::slice::Iter<'a, (RecordReference, RawComponent)>,
}

impl<'a> Iterator for Iter<'a> {
    type Item = (RecordReference, &'a RawComponent);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(id, comp)| (*id, comp))
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<'a> ExactSizeIterator for Iter<'a> {
    #[inline]
    fn len(&self) -> usize {
        self.inner.len()
    }
}

impl<'a> FusedIterator for Iter<'a> {}

impl Encode for Components {
    fn encode<W>(&self, mut writer: W)
    where
        W: Writer,
    {
        (self.components.len() as u64).encode(&mut writer);

        for (id, component) in &self.components {
            id.encode(&mut writer);

            (component.bytes.len() as u64).encode(&mut writer);
            (component.fields.len() as u64).encode(&mut writer);

            component.encode(&mut writer);
        }
    }
}

impl Decode for Components {
    type Error = DecodeError;

    fn decode<R>(mut reader: R) -> Result<Self, Self::Error>
    where
        R: crate::encoding::Reader,
    {
        let len = u64::decode(&mut reader)?;

        let mut components = Components::new();
        for _ in 0..len {
            let id = RecordReference::decode(&mut reader)?;

            let num_bytes = u64::decode(&mut reader)?;
            let num_fields = u64::decode(&mut reader)?;

            let mut bytes = Vec::new();
            let mut fields = Vec::new();

            bytes.extend(&reader.chunk()[..num_bytes as usize]);
            reader.advance(num_bytes as usize);

            let mut starting_offset = None;
            for _ in 0..num_fields {
                let field = reader.next_field().unwrap();
                let starting_offset = starting_offset.get_or_insert(field.offset);

                fields.push(Field {
                    primitive: field.primitive,
                    offset: field.offset - *starting_offset,
                });
            }

            let mut component_reader = BinaryReader::new(bytes.into(), fields.into());
            let component = RawComponent::decode(&mut component_reader)?;
            components.components.push((id, component));
        }

        Ok(components)
    }
}

/// A byte buffer containing component data.
///
/// Note that the buffer has the alignment of `u8`. If you read values from the buffer you must use
/// [`read_unaligned`].
///
/// [`read_unaligned`]: ptr::read_unaligned
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RawComponent {
    bytes: Vec<u8>,
    fields: Vec<Field>,
}

impl RawComponent {
    #[inline]
    pub(crate) const fn empty() -> Self {
        Self {
            bytes: Vec::new(),
            fields: Vec::new(),
        }
    }

    #[inline]
    pub(crate) fn new(bytes: Vec<u8>, fields: Vec<Field>) -> Self {
        Self { bytes, fields }
    }

    pub fn reader(self) -> BinaryReader {
        BinaryReader::new(self.bytes, self.fields.into())
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    #[inline]
    pub fn as_ptr(&self) -> *const u8 {
        self.bytes.as_ptr()
    }

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

    pub fn write<T>(&mut self, value: T)
    where
        T: NoUninit,
    {
        let arr = &[value];
        let slice: &[u8] = bytemuck::cast_slice(arr);

        self.bytes.resize(slice.len(), 0);

        assert!(self.bytes.len() >= slice.len());

        unsafe {
            let dst = self.bytes.as_mut_ptr();
            let src = slice.as_ptr();
            let count = slice.len();

            core::ptr::copy_nonoverlapping(src, dst, count);
        }
    }

    pub unsafe fn write_unchecked<T>(&mut self, value: T)
    where
        T: NoUninit,
    {
        if T::IS_ZST {
            return;
        }

        let slice = bytemuck::bytes_of(&value);

        unsafe {
            let dst = self.bytes.as_mut_ptr();
            let src = slice.as_ptr();
            let count = slice.len();

            core::ptr::copy_nonoverlapping(src, dst, count);
        }
    }

    pub fn update<T, U, F>(&mut self, f: F) -> U
    where
        T: Pod,
        F: FnOnce(&mut T) -> U,
    {
        if T::IS_ZST {
            // Any correctly aligned non-zero pointer is valid for ZST `T`s.
            let mut ptr = NonNull::<T>::dangling();
            let val = unsafe { ptr.as_mut() };
            return f(val);
        }

        assert!(self.bytes.len() >= mem::size_of::<T>());

        // If the buffer is already correctly aligned for `T` we can just
        // cast the pointer into `self.bytes` to `T`.
        // Otherwise we need to copy and write back the value.

        // Also note that some `T`s are always aligned.

        let ptr = self.bytes.as_mut_ptr();

        if ptr.align_offset(mem::align_of::<T>()) == 0 {
            let value = unsafe { &mut *(ptr as *mut T) };
            f(value)
        } else {
            let mut value = unsafe { self.read_unchecked() };
            let res = f(&mut value);
            unsafe { self.write_unchecked(value) };
            res
        }
    }
}

impl AsRef<[u8]> for RawComponent {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        &self.bytes
    }
}

impl Encode for RawComponent {
    fn encode<W>(&self, mut writer: W)
    where
        W: Writer,
    {
        for (index, field) in self.fields.iter().enumerate() {
            let start = field.offset;
            let end = self
                .fields
                .get(index + 1)
                .map(|f| f.offset)
                .unwrap_or(self.bytes.len());

            writer.write(field.primitive, &self.bytes[start..end]);
        }
    }
}

impl Decode for RawComponent {
    type Error = DecodeError;

    fn decode<R>(mut reader: R) -> Result<Self, Self::Error>
    where
        R: crate::encoding::Reader,
    {
        let bytes = reader.chunk().to_vec();
        let mut fields: Vec<Field> = Vec::new();
        while let Some(field) = reader.next_field() {
            fields.push(field);
        }

        Ok(Self { bytes, fields })
    }
}

trait IsZst {
    const IS_ZST: bool;
}

impl<T> IsZst for T {
    const IS_ZST: bool = mem::size_of::<Self>() == 0;
}

pub trait Component: Encode + Decode {
    const ID: RecordReference;
}

#[cfg(test)]
mod tests {
    use core::mem;

    use alloc::vec;
    use alloc::vec::Vec;
    use bytemuck::{Pod, Zeroable};

    use crate::encoding::{BinaryReader, BinaryWriter, Decode, Encode};
    use crate::entity::EntityId;
    use crate::record::{ModuleId, RecordId, RecordReference};

    use super::{Component, Components, RawComponent};

    #[test]
    fn component_update_zst() {
        #[derive(Copy, Clone, Debug, Zeroable, Pod)]
        #[repr(transparent)]
        struct Target;

        let mut component = RawComponent::empty();
        component.update::<Target, _, _>(|val| {
            *val = Target;
        });

        assert_eq!(component.bytes, vec![]);
    }

    #[test]
    fn component_update_aligned() {
        #[derive(Copy, Clone, Debug, Zeroable, Pod)]
        #[repr(C, align(1))]
        struct Target(u8);

        let mut component = RawComponent {
            bytes: vec![0],
            fields: Vec::new(),
        };
        assert!(
            component
                .bytes
                .as_ptr()
                .align_offset(mem::align_of::<Target>())
                == 0
        );

        component.update::<Target, _, _>(|val| {
            *val = Target(1);
        });

        assert_eq!(component.bytes, vec![1]);
    }

    #[test]
    fn component_update_not_aligned() {
        #[derive(Copy, Clone, Debug, Zeroable, Pod)]
        #[repr(C, align(8))]
        struct Target([u8; 32]);

        // If the buffer is aligned, manually "unalign" it by moving the pointer 1 byte
        // forward.
        let mut buf = vec![0; 64];
        let is_aligned = buf.as_ptr().align_offset(mem::align_of::<Target>()) == 0;
        if is_aligned {
            // TODO: Can use `Vec::into_raw_parts` once stable.
            let ptr = buf.as_mut_ptr();
            let len = buf.len();
            let cap = buf.capacity();

            mem::forget(buf);

            buf = unsafe { Vec::from_raw_parts(ptr.add(1), len - 1, cap - 1) };
        }

        let mut component = RawComponent {
            bytes: buf,
            fields: Vec::new(),
        };
        assert!(
            component
                .bytes
                .as_ptr()
                .align_offset(mem::align_of::<Target>())
                != 0
        );

        component.update::<Target, _, _>(|val| {
            *val = Target([1; 32]);
        });

        // If the buffer was orignally aligned we have to truncate the first
        // byte.
        let mut output = if is_aligned { vec![0; 63] } else { vec![0; 64] };
        for index in 0..32 {
            output[index] = 1;
        }

        assert_eq!(component.bytes, output);

        // Drop the orignal buffer so miri shuts up about leaks.
        if is_aligned {
            let ptr = component.bytes.as_mut_ptr();
            let len = component.bytes.len();
            let cap = component.bytes.capacity();

            mem::forget(component);

            drop(unsafe { Vec::from_raw_parts(ptr.sub(1), len + 1, cap + 1) });
        };
    }

    #[derive(Copy, Clone, Debug, Encode, Decode)]
    struct TestComponent {
        a: u8,
        b: u16,
        c: i32,
        d: EntityId,
    }

    impl Component for TestComponent {
        const ID: RecordReference = RecordReference {
            module: ModuleId::CORE,
            record: RecordId(1),
        };
    }

    #[derive(Copy, Clone, Debug, Encode, Decode)]
    struct TestComponent2 {
        a: EntityId,
        b: i32,
        c: u16,
        d: u8,
    }

    impl Component for TestComponent2 {
        const ID: RecordReference = RecordReference {
            module: ModuleId::CORE,
            record: RecordId(2),
        };
    }

    #[test]
    fn raw_component_encode_decode() {
        let component = TestComponent {
            a: 12,
            b: 23456,
            c: -3553512,
            d: EntityId::from_raw(12345),
        };

        let (fields, bytes) = BinaryWriter::new().encoded(&component);

        let raw = RawComponent {
            bytes: bytes.clone(),
            fields: fields.clone(),
        };

        let (raw_fields, raw_bytes) = BinaryWriter::new().encoded(&raw);

        assert_eq!(fields, raw_fields);
        assert_eq!(bytes, raw_bytes);

        let reader = BinaryReader::new(bytes.into(), fields.into());
        let output = RawComponent::decode(reader).unwrap();

        assert_eq!(raw, output);
    }

    #[test]
    fn components_encode_decode() {
        let mut components = Components::new();
        components.insert(TestComponent {
            a: 12,
            b: 23456,
            c: -3553512,
            d: EntityId::from_raw(12345),
        });
        components.insert(TestComponent2 {
            a: EntityId::from_raw(12345),
            b: -3553512,
            c: 23456,
            d: 12,
        });

        let (fields, bytes) = BinaryWriter::new().encoded(&components);

        let reader = BinaryReader::new(bytes.into(), fields.into());
        let output = Components::decode(reader).unwrap();

        assert_eq!(components, output);
    }
}
