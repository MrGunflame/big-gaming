use game_core::hierarchy::Hierarchy;

use crate::GltfNode;

#[derive(Clone, Debug)]
pub struct GltfScene {
    pub nodes: Hierarchy<GltfNode>,
}
