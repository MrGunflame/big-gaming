use std::sync::Arc;

use forward_pass::ForwardPass;
use post_process::PostProcessPass;

use crate::api::CommandQueue;
use crate::forward::ForwardPipeline;
use crate::graph::{Node, NodeLabel, RenderGraph, SlotFlags, SlotKind, SlotLabel};

pub mod forward_pass;
pub mod post_process;

const HDR_TEXTURE: SlotLabel = SlotLabel::new("HDR_TEXTURE");

const SURFACE_INJECTOR: NodeLabel = NodeLabel::new("SURFACE_INJECTOR");

const FORWARD_PASS: NodeLabel = NodeLabel::new("FORWARD_PASS");
const POST_PROCESS_PASS: NodeLabel = NodeLabel::new("POST_PROCESS_PASS");

/// The label of the node that finalizes the rendering process and writes the final image to the
/// surface texture.
pub const FINAL_RENDER_PASS: NodeLabel = POST_PROCESS_PASS;

pub fn init(graph: &mut RenderGraph, forward: Arc<ForwardPipeline>, queue: &mut CommandQueue<'_>) {
    let forward_pass = ForwardPass::new(queue, forward, HDR_TEXTURE);
    let post_process = PostProcessPass::new(queue, HDR_TEXTURE, SlotLabel::SURFACE);

    // `SurfaceInjector` is dummy node that only exists to
    // "inject" the surface texture into the pipeline.
    // The actual surface is provided when before any node is run.
    graph.add_node(SURFACE_INJECTOR, SurfaceInjector);
    graph.add_slot_dependency(
        SURFACE_INJECTOR,
        SlotLabel::SURFACE,
        SlotKind::Texture,
        SlotFlags::WRITE,
    );

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
        SlotFlags::WRITE | SlotFlags::READ,
    );
}

struct SurfaceInjector;

impl Node for SurfaceInjector {
    fn render(&self, _: &mut crate::graph::RenderContext<'_, '_>) {}
}
