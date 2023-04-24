use std::convert::Infallible;

use bytes::{Buf, BufMut};
use game_common::components::actions::Actions;
use game_common::components::components::RecordReference;
use game_common::components::items::{Item, ItemId, ItemStack};
use game_common::components::properties::Properties;
use game_common::id::WeakId;
use game_common::units::Mass;

use super::{Decode, Encode, EofError};

impl Encode for ItemId {
    type Error = Infallible;

    #[inline]
    fn encode<B>(&self, buf: B) -> Result<(), Self::Error>
    where
        B: BufMut,
    {
        self.0.encode(buf)
    }
}

impl Decode for ItemId {
    type Error = EofError;

    #[inline]
    fn decode<B>(buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        RecordReference::decode(buf).map(Self)
    }
}

impl Encode for Item {
    type Error = Infallible;

    fn encode<B>(&self, mut buf: B) -> Result<(), Self::Error>
    where
        B: BufMut,
    {
        self.id.encode(&mut buf)?;
        Ok(())
    }
}

// impl Decode for Item {
//     type Error = EofError;

//     fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
//     where
//         B: Buf,
//     {
//         let id = ItemId::decode(&mut buf)?;

//         Ok(Self {
//             id,
//             resistances: None,
//             mass: Mass::default(),
//             properties: Properties::default(),
//             actions: Actions::default(),
//         })
//     }
// }

// impl Encode for ItemStack {
//     type Error = Infallible;

//     fn encode<B>(&self, mut buf: B) -> Result<(), Self::Error>
//     where
//         B: BufMut,
//     {
//         self.item.encode(&mut buf)?;
//         self.quantity.encode(&mut buf)?;
//         Ok(())
//     }
// }

// impl Decode for ItemStack {
//     type Error = EofError;

//     fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
//     where
//         B: Buf,
//     {
//         let item = Item::decode(&mut buf)?;
//         let quantity = u32::decode(&mut buf)?;

//         Ok(Self { item, quantity })
//     }
// }
