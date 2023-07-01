use std::convert::Infallible;

use super::sequence::Sequence;
use super::{Decode, Encode, Error};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct Ack {
    /// The last acknowledged sequence number + 1.
    ///
    /// In other words the first sequence number that has not yet been received.
    pub sequence: Sequence,

    /// Sequence number of the ACK.
    pub ack_sequence: Sequence,
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct Nak {
    /// The lost sequence number.
    pub sequence: Sequence,
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct AckAck {
    pub ack_sequence: Sequence,
}
