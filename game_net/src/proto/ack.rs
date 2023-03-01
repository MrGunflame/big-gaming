use super::sequence::Sequence;

#[derive(Copy, Clone, Debug)]
pub struct Ack {
    /// The last acknowledged sequence number.
    pub sequence: Sequence,
}

#[derive(Copy, Clone, Debug)]
pub struct Nak {
    /// The lost sequence number.
    pub sequence: Sequence,
}
