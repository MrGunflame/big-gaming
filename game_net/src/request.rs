use game_common::world::control_frame::ControlFrame;

use crate::proto::sequence::Sequence;
use crate::proto::Frame;
use crate::snapshot::CommandId;

#[derive(Clone, Debug)]
pub struct Request {
    pub id: CommandId,
    pub sequence: Sequence,
    pub control_frame: ControlFrame,
    pub frame: Frame,
}
