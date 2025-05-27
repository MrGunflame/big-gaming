//! Main opaque geometry pass using the vertex shader based pipeline

use std::sync::Arc;

use drawcall_gen::DrawcallGenPass;
use forward::OpaqueVertexForwardPass;
use game_render::api::CommandQueue;
use game_render::graph::{NodeLabel, RenderGraph, SlotFlags, SlotKind, SlotLabel};
use parking_lot::Mutex;

use super::{HDR_TEXTURE, State, UPDATE_PASS};

pub mod state;

mod drawcall_gen;
mod forward;

const DRAWCALL_GEN_PASS: NodeLabel = NodeLabel::new("opaque_vertex_drawcall_gen_pass");
const FORWARD_PASS: NodeLabel = NodeLabel::new("opaque_vertex_forward_pass");

const INSTANCE_BUFFER: SlotLabel = SlotLabel::new("opaque_vertex_instance_buffer");
const INDIRECT_DRAW_BUFFER: SlotLabel = SlotLabel::new("opaque_vertex_indirect_draw_buffer");

pub fn init(graph: &mut RenderGraph, queue: &CommandQueue<'_>, state: Arc<Mutex<State>>) {
    let drawcall_gen_pass = DrawcallGenPass::new(queue, state.clone());
    graph.add_node(DRAWCALL_GEN_PASS, drawcall_gen_pass);

    graph.add_node_dependency(DRAWCALL_GEN_PASS, UPDATE_PASS);

    let forward_pass = OpaqueVertexForwardPass::new(queue, state);
    graph.add_node(FORWARD_PASS, forward_pass);

    graph.add_slot_dependency(
        DRAWCALL_GEN_PASS,
        INSTANCE_BUFFER,
        SlotKind::Buffer,
        SlotFlags::WRITE,
    );
    graph.add_slot_dependency(
        DRAWCALL_GEN_PASS,
        INDIRECT_DRAW_BUFFER,
        SlotKind::Buffer,
        SlotFlags::WRITE,
    );

    graph.add_slot_dependency(
        FORWARD_PASS,
        INSTANCE_BUFFER,
        SlotKind::Buffer,
        SlotFlags::READ,
    );
    graph.add_slot_dependency(
        FORWARD_PASS,
        INDIRECT_DRAW_BUFFER,
        SlotKind::Buffer,
        SlotFlags::READ,
    );
    graph.add_slot_dependency(
        FORWARD_PASS,
        HDR_TEXTURE,
        SlotKind::Texture,
        SlotFlags::WRITE,
    );
    graph.add_slot_dependency(
        FORWARD_PASS,
        SlotLabel::SURFACE,
        SlotKind::Texture,
        SlotFlags::READ,
    );
}
