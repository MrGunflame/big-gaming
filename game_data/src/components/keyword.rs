use bytes::{Buf, BufMut};
use game_common::module::ModuleId;

use crate::{Decode, Encode};

#[derive(Clone, Debug)]
pub struct Keyword {
    pub module: ModuleId,
    pub id: KeywordId,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Operation {
    /// Add a keyword to a template.
    Add,
    /// Removes a keyworld from an template.
    Remove,
}

impl Encode for Operation {
    fn encode<B>(&self, buf: B)
    where
        B: BufMut,
    {
        let n: u8 = match self {
            Self::Add => 0,
            Self::Remove => 0,
        };

        n.encode(buf);
    }
}

impl Decode for Operation {
    type Error = std::io::Error;

    fn decode<B>(buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let n = u8::decode(buf)?;

        match n {
            0 => Ok(Self::Add),
            1 => Ok(Self::Remove),
            _ => panic!("invalid operation"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct KeywordId;
