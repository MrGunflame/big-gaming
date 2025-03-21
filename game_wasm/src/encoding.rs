use bytes::Buf;
pub use game_macros::{wasm__decode as Decode, wasm__encode as Encode};

use core::mem::MaybeUninit;

use alloc::collections::VecDeque;
use alloc::vec::Vec;
use glam::{Quat, Vec2, Vec3, Vec4};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Primitive {
    Bytes,
    EntityId,
    PlayerId,
    RuntimeResourceId,
}

impl Primitive {
    pub(crate) fn from_u8(tag: u8) -> Option<Self> {
        match tag {
            0 => Some(Self::Bytes),
            1 => Some(Self::EntityId),
            2 => Some(Self::PlayerId),
            3 => Some(Self::RuntimeResourceId),
            _ => None,
        }
    }

    pub(crate) fn to_u8(self) -> u8 {
        match self {
            Self::Bytes => 0,
            Self::EntityId => 1,
            Self::PlayerId => 2,
            Self::RuntimeResourceId => 3,
        }
    }
}

/// A type that can be encoded into an arbitrary bytestream.
pub trait Encode {
    /// Encodes this value into the given [`Writer`].
    fn encode<W>(&self, writer: W)
    where
        W: Writer;
}

/// A type that can be decoded from an arbitrary bytestream.
pub trait Decode: Sized {
    /// An error that can occur while decoding this value.
    type Error: core::fmt::Debug;

    /// Decodes this value from the given [`Reader`].
    ///
    /// # Errors
    ///
    /// Returns an `Error` if decoding fails for any reason:
    /// - The reader has no more bytes, but the value was not fully decoded.
    /// - The reader contained bytes invalid for this value.
    fn decode<R>(reader: R) -> Result<Self, Self::Error>
    where
        R: Reader;
}

pub trait Writer {
    fn write(&mut self, primitive: Primitive, data: &[u8]);
}

impl<W> Writer for &mut W
where
    W: Writer,
{
    #[inline]
    fn write(&mut self, primitive: Primitive, data: &[u8]) {
        W::write(self, primitive, data)
    }
}

pub trait Reader: core::fmt::Debug {
    fn next(&mut self) -> Option<Primitive>;
    fn chunk(&self) -> &[u8];
    fn advance(&mut self, count: usize);

    fn peek(&self) -> Option<Primitive>;

    fn next_field(&mut self) -> Option<Field>;
}

impl<R> Reader for &mut R
where
    R: Reader,
{
    #[inline]
    fn next(&mut self) -> Option<Primitive> {
        R::next(self)
    }

    #[inline]
    fn chunk(&self) -> &[u8] {
        R::chunk(self)
    }

    #[inline]
    fn advance(&mut self, count: usize) {
        R::advance(self, count)
    }

    #[inline]
    fn peek(&self) -> Option<Primitive> {
        R::peek(self)
    }

    #[inline]
    fn next_field(&mut self) -> Option<Field> {
        R::next_field(self)
    }
}

pub struct BinaryWriter {
    primitives: Vec<Field>,
    buffer: Vec<u8>,
}

impl BinaryWriter {
    pub fn new() -> Self {
        Self {
            primitives: Vec::new(),
            buffer: Vec::new(),
        }
    }

    pub fn encoded<T>(mut self, t: &T) -> (Vec<Field>, Vec<u8>)
    where
        T: Encode,
    {
        t.encode(&mut self);
        (self.primitives, self.buffer)
    }
}

impl Writer for BinaryWriter {
    fn write(&mut self, primitive: Primitive, data: &[u8]) {
        let offset = self.buffer.len();
        self.primitives.push(Field { primitive, offset });
        self.buffer.extend(data);
    }
}

impl Default for BinaryWriter {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct BinaryReader {
    fields: VecDeque<Field>,
    data: Vec<u8>,
    start: usize,
}

impl Reader for BinaryReader {
    fn next(&mut self) -> Option<Primitive> {
        self.fields.pop_front().map(|f| f.primitive)
    }

    fn chunk(&self) -> &[u8] {
        &self.data[self.start..]
    }

    fn advance(&mut self, count: usize) {
        self.start += count;
    }

    fn peek(&self) -> Option<Primitive> {
        self.fields.front().map(|f| f.primitive)
    }

    fn next_field(&mut self) -> Option<Field> {
        self.fields.pop_front()
    }
}

impl BinaryReader {
    pub fn new(data: Vec<u8>, fields: VecDeque<Field>) -> Self {
        Self {
            fields,
            data,
            start: 0,
        }
    }
}

impl<T, const N: usize> Encode for [T; N]
where
    T: Encode,
{
    fn encode<W>(&self, mut writer: W)
    where
        W: Writer,
    {
        for elem in self {
            elem.encode(&mut writer);
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DecodeError {
    Eof {
        ident: &'static str,
        expected: usize,
        found: usize,
    },
    InvalidPrimitive {
        ident: &'static str,
        expected: Primitive,
        found: Primitive,
    },
    InvalidVariant {
        ident: &'static str,
        value: u64,
    },
    InvalidString,
}

impl<T, const N: usize> Decode for [T; N]
where
    T: Decode,
    DecodeError: From<T::Error>,
{
    type Error = DecodeError;

    fn decode<R>(mut reader: R) -> Result<Self, Self::Error>
    where
        R: Reader,
    {
        let mut array: [MaybeUninit<T>; N] = unsafe { MaybeUninit::uninit().assume_init() };
        let mut len = 0;

        struct DropGuard<'a, T>(&'a mut [MaybeUninit<T>]);

        impl<'a, T> Drop for DropGuard<'a, T> {
            #[inline(never)]
            #[cold]
            fn drop(&mut self) {
                for elem in &mut *self.0 {
                    unsafe {
                        elem.assume_init_drop();
                    }
                }
            }
        }

        for index in 0..N {
            let guard = DropGuard(&mut array[..len]);
            let elem = T::decode(&mut reader)?;
            core::mem::forget(guard);

            array[index].write(elem);
            len += 1;
        }

        let array = unsafe { core::mem::transmute_copy::<[MaybeUninit<T>; N], [T; N]>(&array) };

        Ok(array)
    }
}

impl Encode for u8 {
    fn encode<W>(&self, mut writer: W)
    where
        W: Writer,
    {
        writer.write(Primitive::Bytes, &[*self]);
    }
}

impl Decode for u8 {
    type Error = DecodeError;

    fn decode<R>(mut reader: R) -> Result<Self, Self::Error>
    where
        R: Reader,
    {
        reader.next();
        // if reader.next() != Some(Primitive::Bytes) {
        //     return Err(DecodeError);
        // }

        if let Some(byte) = reader.chunk().first().copied() {
            reader.advance(1);
            Ok(byte)
        } else {
            Err(DecodeError::Eof {
                ident: stringify!(u8),
                expected: 1,
                found: 0,
            })
        }
    }
}

macro_rules! impl_primitive {
    ($($t:ty),*) => {
        $(
            impl Encode for $t {
                #[inline]
                fn encode<W>(&self, writer: W)
                where
                    W: Writer,
                {
                    self.to_le_bytes().encode(writer);
                }
            }

            impl Decode for $t {
                type Error = DecodeError;

                #[inline]
                fn decode<R>(reader: R) -> Result<Self, Self::Error>
                where
                    R: Reader,
                {
                    <[u8; core::mem::size_of::<Self>()]>::decode(reader).map(Self::from_le_bytes)
                }
            }
        )*
    };
}

impl_primitive! { u16, u32, u64, u128, i8, i16, i32, i64, i128, f32, f64 }

macro_rules! impl_as_array {
    ($($t:ty),*) => {
        $(
            impl Encode for $t {
                #[inline]
                fn encode<W>(&self, writer: W)
                where
                    W: Writer,
                {
                    self.to_array().encode(writer);
                }
            }

            impl Decode for $t {
                type Error = DecodeError;

                fn decode<R>(reader: R) -> Result<Self, Self::Error>
                where
                    R: Reader,
                {
                    Decode::decode(reader).map(Self::from_array)
                }
            }
        )*
    };
}

impl_as_array! { Vec2, Vec3, Vec4, Quat }

impl Encode for bool {
    fn encode<W>(&self, writer: W)
    where
        W: Writer,
    {
        (*self as u8).encode(writer);
    }
}

impl Decode for bool {
    type Error = <u8 as Decode>::Error;

    fn decode<R>(reader: R) -> Result<Self, Self::Error>
    where
        R: Reader,
    {
        u8::decode(reader).map(|b| b != 0)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Field {
    pub primitive: Primitive,
    pub offset: usize,
}

impl Field {
    pub const ENCODED_SIZE: usize = 1 + 4;
}

/// Returns `(data, fields)`.
pub fn encode_value<T>(value: &T) -> (Vec<u8>, Vec<u8>)
where
    T: Encode,
{
    let writer = BinaryWriter::new();
    let (fields, data) = writer.encoded(value);

    (data, encode_fields(&fields))
}

pub fn decode_fields(mut buf: &[u8]) -> Vec<Field> {
    let mut fields = Vec::new();
    while !buf.is_empty() {
        let primitive = Primitive::from_u8(buf.get_u8()).unwrap();
        // usize == u32 for wasm32 arch.
        let offset = buf.get_u32_le() as usize;
        fields.push(Field { primitive, offset });
    }

    fields
}

pub fn encode_fields(fields: &[Field]) -> Vec<u8> {
    let mut fields_encoded = Vec::new();
    for field in fields {
        fields_encoded.push(field.primitive.to_u8());
        fields_encoded.extend((field.offset as u32).to_le_bytes());
    }
    fields_encoded
}

impl<T> Encode for Vec<T>
where
    T: Encode,
{
    fn encode<W>(&self, mut writer: W)
    where
        W: Writer,
    {
        (self.len() as u64).encode(&mut writer);
        for elem in self {
            elem.encode(&mut writer);
        }
    }
}

impl<T> Decode for Vec<T>
where
    T: Decode,
    DecodeError: From<T::Error>,
{
    type Error = DecodeError;

    fn decode<R>(mut reader: R) -> Result<Self, Self::Error>
    where
        R: Reader,
    {
        let len = u64::decode(&mut reader)?;
        let mut elems = Vec::with_capacity(len as usize);
        for _ in 0..len {
            elems.push(T::decode(&mut reader)?);
        }
        Ok(elems)
    }
}
