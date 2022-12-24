use std::convert::Infallible;

use bevy_rapier3d::dynamics::Velocity;
use bytes::{Buf, BufMut};

use super::{Decode, Encode};

impl Encode for Velocity {
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        self.linvel.encode(&mut buf);
        self.angvel.encode(&mut buf);
    }
}

impl Decode for Velocity {
    type Error = Infallible;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let linvel = Decode::decode(&mut buf)?;
        let angvel = Decode::decode(&mut buf)?;

        Ok(Self { linvel, angvel })
    }
}
