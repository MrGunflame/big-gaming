use crate::graph::{Node, NodeLabel, RenderGraph, SlotFlags, SlotKind, SlotLabel};

const SURFACE_INJECTOR: NodeLabel = NodeLabel::new("SURFACE_INJECTOR");

pub fn init(graph: &mut RenderGraph) {
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
}

struct SurfaceInjector;

impl Node for SurfaceInjector {
    fn render(&self, _: &mut crate::graph::RenderContext<'_, '_>) {}
}
