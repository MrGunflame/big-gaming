pub(crate) mod pipeline;

use game_common::components::Color;
use glam::Vec3;

#[derive(Copy, Clone, Debug)]
pub(crate) struct DrawCommand {
    pub(crate) start: Vec3,
    pub(crate) end: Vec3,
    pub(crate) color: Color,
}
