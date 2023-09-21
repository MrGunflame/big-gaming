use game_common::world::control_frame::ControlFrame;

use crate::proto::sequence::Sequence;
use crate::proto::Frame;

#[derive(Clone, Debug)]
pub struct Request {
    pub sequence: Sequence,
    pub control_frame: ControlFrame,
    pub frame: Frame,
}
