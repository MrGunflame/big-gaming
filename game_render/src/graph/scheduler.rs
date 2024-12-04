use std::collections::HashMap;

use thiserror::Error;

use super::{Dependency, NodeLabel, RenderGraph, SlotFlags, SlotLabel};

pub struct RenderGraphScheduler;

impl RenderGraphScheduler {
    pub fn schedule(&mut self, graph: &RenderGraph) -> Result<Vec<NodeLabel>, ScheduleError> {
        let mut write_slots: HashMap<&SlotLabel, Vec<_>> = HashMap::new();
        let mut read_slots: HashMap<&SlotLabel, Vec<_>> = HashMap::new();

        // Dependencies that must always be satisfied
        // regardless of permutation.
        let mut fixed_dependencies = HashMap::new();

        for node in graph.nodes.values() {
            let mut node_deps = Vec::new();

            for dep in &node.dependencies {
                match dep {
                    Dependency::Node(label) => {
                        node_deps.push(label);
                    }
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

            fixed_dependencies.insert(node.label, node_deps);
        }

        let mut dependency_options: HashMap<NodeLabel, HashMap<&SlotLabel, Vec<&NodeLabel>>> =
            HashMap::new();

        for node in graph.nodes.values() {
            for dep in &node.dependencies {
                match dep {
                    Dependency::Node(_) => {}
                    Dependency::Slot(label, kind, flags) => {
                        // Requires that another node that has written to
                        // this slot is ran before.
                        if flags.contains(SlotFlags::READ) {
                            let src = write_slots
                                .get(&label)
                                .unwrap()
                                .iter()
                                // If the node reads and writes to a slot it
                                // must not be scheduled as its own predecessor.
                                .filter(|src| **src != node.label)
                                // If the node that provides the dependency is always
                                // scheduled after the current node it cannot possibly
                                // provide the slot for this node.
                                .filter(|src| match fixed_dependencies.get(&src) {
                                    Some(nodes) => !nodes.contains(&&node.label),
                                    None => true,
                                })
                                .collect::<Vec<_>>();

                            // Cannot schedule the node if no nodes providing the
                            // required dependencies exist.
                            if src.is_empty() {
                                return Err(ScheduleError::NoSource(node.label, *label));
                            }

                            // If only a single possible node exists we already
                            // have found the only possible option.
                            if src.len() == 1 {
                                // If the node reads and writes to a slot it
                                // must not be scheduled as its own predecessor.
                                debug_assert_ne!(&node.label, src[0]);

                                fixed_dependencies
                                    .entry(node.label)
                                    .or_default()
                                    .push(&src[0]);
                                continue;
                            }

                            dependency_options
                                .entry(node.label)
                                .or_default()
                                .insert(label, src);
                        }
                    }
                }
            }
        }

        // Invert the `dependency_options` strucuture and create a
        // set of permutations to check.
        let mut permutations = PermutationSet::new();
        for (node, slots) in &dependency_options {
            for opts in slots.values() {
                let mut values = Vec::new();

                for opt in opts {
                    values.push(DependencyOption {
                        node,
                        depends_on: opt,
                    });
                }

                permutations.add(&values);
            }
        }

        let mut permutations_iter = permutations.iter();

        let mut output = Vec::new();
        loop {
            output.clear();

            let mut dependency_list = fixed_dependencies.clone();
            match permutations_iter.next() {
                Some(deps) => {
                    for dep in deps {
                        let node_deps = dependency_list.entry(*dep.node).or_default();

                        // The dependency list should never contain a node more than once.
                        // If it already exists the dependency is already satisfied and
                        // we don't have to add it.
                        if !node_deps.contains(&dep.node) {
                            node_deps.push(dep.depends_on);
                        }
                    }
                }
                // If we have no permutations ignore the permutations iterator
                // entirely. We always have a fixed pipeline.
                None if permutations.is_empty() => (),
                // If we reach this point we have tried all possible
                // permutations and none of them are valid.
                None => return Err(ScheduleError::NoValidPermutation),
            }

            while !dependency_list.is_empty() {
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
                        for dependencies in dependency_list.values_mut() {
                            dependencies.retain(|dep| **dep != node);
                        }

                        output.push(node);
                    }
                    // No node could be scheduled this iteration.
                    // This means we have a cycle.
                    None => return Err(ScheduleError::Cycle),
                }
            }

            return Ok(output);
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Error)]
pub enum ScheduleError {
    #[error("cycle detected")]
    Cycle,
    #[error("no source for slot {1:?} for node {0:?}")]
    NoSource(NodeLabel, SlotLabel),
    #[error("no valid permutation for slot configuration")]
    NoValidPermutation,
}

#[derive(Copy, Clone, Debug)]
struct DependencyOption<'a> {
    node: &'a NodeLabel,
    depends_on: &'a NodeLabel,
}

/// A set that mixes sets of possible permutations together.
#[derive(Clone, Debug)]
struct PermutationSet<T> {
    sets: Vec<Vec<T>>,
}

impl<T> PermutationSet<T>
where
    T: Copy,
{
    fn new() -> Self {
        Self { sets: Vec::new() }
    }

    fn is_empty(&self) -> bool {
        self.sets.is_empty()
    }

    /// Adds a new set of possible values to the set.
    fn add(&mut self, values: &[T]) {
        if self.sets.is_empty() {
            for value in values {
                self.sets.push(vec![*value]);
            }
        } else {
            let mut new_sets = Vec::new();
            for value in values {
                for set in &self.sets {
                    let mut new_set = set.clone();
                    new_set.push(*value);
                    new_sets.push(new_set);
                }
            }

            self.sets = new_sets;
        }
    }

    /// Returns an iterator over all possible permutations in this set.
    fn iter(&self) -> impl Iterator<Item = &[T]> {
        self.sets.iter().map(|v| v.as_slice())
    }
}

#[cfg(test)]
mod test {
    use crate::graph::{
        Node, NodeLabel, RenderContext, RenderGraph, SlotFlags, SlotKind, SlotLabel,
    };

    use super::{PermutationSet, RenderGraphScheduler, ScheduleError};

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

    #[test]
    fn render_graph_read_write_slot() {
        let a = NodeLabel::new("A");
        let b = NodeLabel::new("B");

        let mut graph = RenderGraph::new();
        graph.add_node(a, TestNode);
        graph.add_node(b, TestNode);

        graph.add_slot_dependency(
            a,
            SlotLabel::new("test"),
            SlotKind::Texture,
            SlotFlags::WRITE,
        );

        graph.add_slot_dependency(
            b,
            SlotLabel::new("test"),
            SlotKind::Buffer,
            SlotFlags::READ | SlotFlags::WRITE,
        );

        let res = RenderGraphScheduler.schedule(&graph).unwrap();
        assert_eq!(res, [a, b]);
    }

    #[test]
    fn render_graph_many_permutations() {
        let a = NodeLabel::new("A");
        let b = NodeLabel::new("B");
        let c = NodeLabel::new("C");
        let d = NodeLabel::new("D");
        let e = NodeLabel::new("E");

        let x = SlotLabel::new("x");
        let y = SlotLabel::new("y");

        let mut graph = RenderGraph::new();
        graph.add_node(a, TestNode);
        graph.add_node(b, TestNode);
        graph.add_node(c, TestNode);
        graph.add_node(d, TestNode);
        graph.add_node(e, TestNode);

        // A | B => C
        // D | E => C
        graph.add_slot_dependency(a, x, SlotKind::Buffer, SlotFlags::WRITE);
        graph.add_slot_dependency(b, x, SlotKind::Buffer, SlotFlags::WRITE);
        graph.add_slot_dependency(c, x, SlotKind::Buffer, SlotFlags::READ);
        graph.add_slot_dependency(c, y, SlotKind::Buffer, SlotFlags::READ);
        graph.add_slot_dependency(d, y, SlotKind::Buffer, SlotFlags::WRITE);
        graph.add_slot_dependency(e, y, SlotKind::Buffer, SlotFlags::WRITE);

        let res = RenderGraphScheduler.schedule(&graph).unwrap();
        let index_a = res.iter().position(|v| *v == a).unwrap();
        let index_b = res.iter().position(|v| *v == b).unwrap();
        let index_c = res.iter().position(|v| *v == c).unwrap();
        let index_d = res.iter().position(|v| *v == d).unwrap();
        let index_e = res.iter().position(|v| *v == e).unwrap();

        // A or B before C
        // D or E before C
        assert!(index_a < index_c || index_b < index_c);
        assert!(index_d < index_c || index_e < index_c);
    }

    #[test]
    fn permutation_set() {
        let mut set = PermutationSet::new();
        set.add(&["A", "B"]);
        assert_eq!(set.iter().collect::<Vec<_>>(), [["A"], ["B"]]);

        set.add(&["X", "Y"]);
        assert_eq!(
            set.iter().collect::<Vec<_>>(),
            [["A", "X"], ["B", "X"], ["A", "Y"], ["B", "Y"]]
        );

        set.add(&["E", "F", "G"]);
        assert_eq!(
            set.iter().collect::<Vec<_>>(),
            [
                ["A", "X", "E"],
                ["B", "X", "E"],
                ["A", "Y", "E"],
                ["B", "Y", "E"],
                ["A", "X", "F"],
                ["B", "X", "F"],
                ["A", "Y", "F"],
                ["B", "Y", "F"],
                ["A", "X", "G"],
                ["B", "X", "G"],
                ["A", "Y", "G"],
                ["B", "Y", "G"],
            ]
        );
    }
}
