use crate::proto::sequence::Sequence;
use crate::proto::Frame;
use crate::snapshot::CommandId;

#[derive(Clone, Debug)]
pub struct Request {
    pub id: CommandId,
    pub sequence: Sequence,
    pub frame: Frame,
}
