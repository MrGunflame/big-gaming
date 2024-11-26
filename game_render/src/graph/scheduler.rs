use std::collections::HashMap;

use super::{Dependency, NodeLabel, RenderGraph, SlotFlags};

pub struct RenderGraphScheduler;

impl RenderGraphScheduler {
    pub fn schedule(&mut self, graph: &RenderGraph) -> Result<Vec<NodeLabel>, ScheduleError> {
        let mut write_slots: HashMap<&super::SlotLabel, Vec<_>> = HashMap::new();
        let mut read_slots: HashMap<&super::SlotLabel, Vec<_>> = HashMap::new();

        let mut dependency_list = HashMap::new();

        for node in graph.nodes.values() {
            for dep in &node.dependencies {
                match dep {
                    Dependency::Node(_) => (),
                    Dependency::Slot(label, kind, flags) => {
                        if flags.contains(SlotFlags::READ) {
                            read_slots.entry(label).or_default().push(node.label);
                        }

                        if flags.contains(SlotFlags::WRITE) {
                            write_slots.entry(label).or_default().push(node.label);
                        }
                    }
                }
            }
        }

        for node in graph.nodes.values() {
            let mut node_deps = Vec::new();

            for dep in &node.dependencies {
                match dep {
                    Dependency::Node(label) => {
                        node_deps.push(label);
                    }
                    Dependency::Slot(label, kind, flags) => {
                        if flags.contains(SlotFlags::READ) {
                            let src = write_slots.get(&label).unwrap();
                            assert!(!src.is_empty());

                            node_deps.extend(src);
                        }
                    }
                }
            }

            dependency_list.insert(node.label, node_deps);
        }

        let mut output = Vec::new();

        loop {
            if dependency_list.is_empty() {
                return Ok(output);
            }

            let mut remove_node = None;

            for (node, dependencies) in dependency_list.iter() {
                if dependencies.is_empty() {
                    remove_node = Some(*node);
                    break;
                }
            }

            match remove_node {
                Some(node) => {
                    dependency_list.remove(&node);
                    for deps in dependency_list.values_mut() {
                        deps.retain(|dep| **dep != node);
                    }
                    output.push(node);
                }
                None => {
                    return Err(ScheduleError::Cycle);
                }
            }
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ScheduleError {
    Cycle,
}

#[cfg(test)]
mod test {
    use crate::graph::{
        Node, NodeLabel, RenderContext, RenderGraph, SlotFlags, SlotKind, SlotLabel,
    };

    use super::{RenderGraphScheduler, ScheduleError};

    struct TestNode;

    impl Node for TestNode {
        fn render(&self, _: &mut RenderContext<'_, '_>) {}
    }

    #[test]
    fn render_graph_schedule_node_dependency() {
        let mut graph = RenderGraph::new();
        graph.add_node(NodeLabel::new("A"), TestNode);
        graph.add_node(NodeLabel::new("B"), TestNode);
        // A runs before B or in other words B requires A.
        graph.add_node_dependency(NodeLabel::new("B"), NodeLabel::new("A"));

        let queue = RenderGraphScheduler.schedule(&graph).unwrap();
        assert_eq!(queue, [NodeLabel::new("A"), NodeLabel::new("B")]);
    }

    #[test]
    fn render_graph_schedule_node_dependency_parallel() {
        let a = NodeLabel::new("A");
        let b = NodeLabel::new("B");
        let c = NodeLabel::new("C");

        let mut graph = RenderGraph::new();
        graph.add_node(a, TestNode);
        graph.add_node(b, TestNode);
        graph.add_node(c, TestNode);

        // B requires A and C to run before.
        graph.add_node_dependency(b, a);
        graph.add_node_dependency(b, c);

        let queue = RenderGraphScheduler.schedule(&graph).unwrap();
        assert!(queue == [a, c, b] || queue == [c, a, b]);
    }

    #[test]
    fn render_graph_schedule_slot_dependency() {
        let a = NodeLabel::new("A");
        let b = NodeLabel::new("B");
        let c = NodeLabel::new("C");

        let mut graph = RenderGraph::new();
        graph.add_node(a, TestNode);
        graph.add_node(b, TestNode);
        graph.add_node(c, TestNode);

        graph.add_slot_dependency(
            b,
            SlotLabel::new("texture"),
            SlotKind::Texture,
            SlotFlags::WRITE,
        );
        graph.add_slot_dependency(
            c,
            SlotLabel::new("texture"),
            SlotKind::Buffer,
            SlotFlags::READ,
        );
        graph.add_slot_dependency(
            c,
            SlotLabel::new("buffer"),
            SlotKind::Buffer,
            SlotFlags::WRITE,
        );
        graph.add_slot_dependency(
            a,
            SlotLabel::new("buffer"),
            SlotKind::Buffer,
            SlotFlags::READ,
        );

        let queue = RenderGraphScheduler.schedule(&graph).unwrap();
        assert_eq!(queue, [b, c, a]);
    }

    #[test]
    fn render_graph_cycle() {
        let a = NodeLabel::new("A");
        let b = NodeLabel::new("B");

        let mut graph = RenderGraph::new();

        graph.add_node(a, TestNode);
        graph.add_node(b, TestNode);

        graph.add_node_dependency(a, b);
        graph.add_node_dependency(b, a);

        let res = RenderGraphScheduler.schedule(&graph);
        assert_eq!(res, Err(ScheduleError::Cycle));
    }
}
