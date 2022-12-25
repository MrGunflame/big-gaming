//! The binary format used for savegames.
//!

use std::convert::Infallible;
use std::mem::{self, MaybeUninit};
use std::ptr;

use bytes::{Buf, BufMut};
mod combat;
mod items;
mod math;
mod physics;
mod transform;

pub struct Error {}

#[derive(Clone, Debug)]
pub struct Header {
    pub version: u8,
    pub checksum: u32,
}

pub trait Encode {
    fn encode<B>(&self, buf: B)
    where
        B: BufMut;
}

pub trait Decode: Sized {
    type Error;

    fn decode<B>(buf: B) -> Result<Self, Self::Error>
    where
        B: Buf;
}

macro_rules! impl_primitive {
    ($($t:ty),*) => {
        $(
            impl Encode for $t {
                #[inline]
                fn encode<B>(&self, mut buf: B)
                where
                    B: BufMut,
                {
                    buf.put_slice(&self.to_le_bytes());
                }
            }

            impl Decode for $t {
                type Error = Infallible;

                fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
                where
                    B: Buf,
                {
                    let mut bytes = [0; mem::size_of::<Self>()];
                    buf.copy_to_slice(&mut bytes);
                    Ok(Self::from_le_bytes(bytes))
                }
            }
        )*
    };
}

impl_primitive!(u8, u16, u32, u64, i8, i16, i32, i64, f32, f64);

impl<T> Encode for [T]
where
    T: Encode,
{
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        for elem in self {
            elem.encode(&mut buf);
        }
    }
}

impl<T, const N: usize> Decode for [T; N]
where
    T: Decode,
{
    type Error = T::Error;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        // SAFETY: `[MaybeUninit<T>]` doesn't require initialization.
        let mut array: [MaybeUninit<T>; N] = unsafe { MaybeUninit::uninit().assume_init() };

        for index in 0..N {
            array[index].write(T::decode(&mut buf)?);
        }

        // SAFETY: All elements of `array` has been fully initialized.
        // Note that this has the same effect as `std::mem::transmute`, but works with const
        // generics.
        Ok(unsafe { ptr::read(array.as_ptr() as *const _) })
    }
}
