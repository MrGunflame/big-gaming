use std::collections::HashMap;
use std::sync::Arc;

use forward_pass::ForwardPass;
use parking_lot::Mutex;
use post_process::PostProcessPass;
use wgpu::Device;

use crate::camera::RenderTarget;
use crate::forward::ForwardPipeline;
use crate::graph::{NodeLabel, RenderGraph, SlotFlags, SlotKind, SlotLabel};
use crate::state::RenderState;

pub mod forward_pass;
pub mod post_process;

const HDR_TEXTURE: SlotLabel = SlotLabel::new("HDR_TEXTURE");

const FORWARD_PASS: NodeLabel = NodeLabel::new("FORWARD_PASS");
const POST_PROCESS_PASS: NodeLabel = NodeLabel::new("POST_PROCESS_PASS");

/// The label of the node that finalizes the rendering process and writes the final image to the
/// surface texture.
pub const FINAL_RENDER_PASS: NodeLabel = POST_PROCESS_PASS;

pub fn init(
    graph: &mut RenderGraph,
    state: Arc<Mutex<HashMap<RenderTarget, RenderState>>>,
    forward: Arc<ForwardPipeline>,
    device: &Device,
) {
    let forward_pass = ForwardPass::new(state, forward, HDR_TEXTURE);
    let post_process = PostProcessPass::new(device, HDR_TEXTURE, SlotLabel::SURFACE);

    graph.add_node(FORWARD_PASS, forward_pass);
    graph.add_slot_dependency(
        FORWARD_PASS,
        HDR_TEXTURE,
        SlotKind::Texture,
        SlotFlags::WRITE,
    );

    graph.add_node(POST_PROCESS_PASS, post_process);
    graph.add_slot_dependency(
        POST_PROCESS_PASS,
        HDR_TEXTURE,
        SlotKind::Texture,
        SlotFlags::READ,
    );
    graph.add_slot_dependency(
        POST_PROCESS_PASS,
        SlotLabel::SURFACE,
        SlotKind::Texture,
        SlotFlags::WRITE,
    );
}
