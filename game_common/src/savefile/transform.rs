//! Entity transform for storing positional data

use std::convert::Infallible;

use bevy_transform::components::Transform;
use bytes::{Buf, BufMut};

use super::{Decode, Encode};

impl Encode for Transform {
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        self.translation.encode(&mut buf);
        self.rotation.encode(&mut buf);
        self.scale.encode(&mut buf);
    }
}

impl Decode for Transform {
    type Error = Infallible;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let translation = Decode::decode(&mut buf)?;
        let rotation = Decode::decode(&mut buf)?;
        let scale = Decode::decode(&mut buf)?;

        Ok(Self {
            translation,
            rotation,
            scale,
        })
    }
}
