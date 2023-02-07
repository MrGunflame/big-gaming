//! Math type impls

use std::convert::Infallible;

use bytes::{Buf, BufMut};
use glam::{Quat, Vec3};

use super::{Decode, Encode};

impl Encode for Vec3 {
    #[inline]
    fn encode<B>(&self, buf: B)
    where
        B: BufMut,
    {
        let slice: &[f32; 3] = self.as_ref();
        slice.encode(buf);
    }
}

impl Decode for Vec3 {
    type Error = Infallible;

    fn decode<B>(buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let array = <[f32; 3]>::decode(buf)?;
        Ok(Self::from_array(array))
    }
}

impl Encode for Quat {
    fn encode<B>(&self, buf: B)
    where
        B: BufMut,
    {
        let slice: &[f32; 4] = self.as_ref();
        slice.encode(buf);
    }
}

impl Decode for Quat {
    type Error = Infallible;

    fn decode<B>(buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let array = <[f32; 4]>::decode(buf)?;
        Ok(Self::from_array(array))
    }
}
