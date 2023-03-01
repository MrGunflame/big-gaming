use std::convert::Infallible;

use super::sequence::Sequence;
use super::{Decode, Encode, Error};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct Ack {
    /// The last acknowledged sequence number.
    pub sequence: Sequence,
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct Nak {
    /// The lost sequence number.
    pub sequence: Sequence,
}
