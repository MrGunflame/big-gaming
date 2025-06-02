mod queue;

use std::hash::Hash;

use allocator_api2::alloc::Allocator;
use allocator_api2::vec::Vec;
use game_tracing::trace_span;
use hashbrown::{HashMap, HashSet};
use nohash_hasher::BuildNoHashHasher;
use queue::Queue;

use crate::backend::AccessFlags;

#[derive(Debug)]
pub struct Scheduler {
    resource_map_cap: usize,
    predecessors_cap: usize,
    sucessors_cap: usize,
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            resource_map_cap: 0,
            predecessors_cap: 0,
            sucessors_cap: 0,
        }
    }

    pub fn schedule<'a, T, M, A>(
        &mut self,
        mut resources: M,
        allocator: A,
        nodes: &'a [T],
    ) -> std::vec::Vec<Step<&'a T, T::ResourceId>>
    where
        T: Node,
        T::ResourceId: Copy + Hash + Eq,
        M: ResourceMap<Id = T::ResourceId>,
        A: Allocator,
    {
        let _span = trace_span!("Scheduler::schedule").entered();

        let mut resource_accesses =
            HashMap::<T::ResourceId, Vec<usize, &A>, _, &A>::with_capacity_in(
                self.resource_map_cap,
                &allocator,
            );
        // We use linear indices as the key, they are already uniformly
        // distributed so we can skip the hashing.
        let mut predecessors =
            Vec::<Option<HashSet<usize, BuildNoHashHasher<usize>, &A>>, &A>::with_capacity_in(
                self.predecessors_cap,
                &allocator,
            );
        predecessors.resize(nodes.len(), None);
        let mut successors =
            Vec::<Vec<usize, &A>, &A>::with_capacity_in(self.sucessors_cap, &allocator);
        successors.resize(nodes.len(), Vec::new_in(&allocator));

        let queue = Queue::new_in(nodes.len(), &allocator);

        for (index, node) in nodes.iter().enumerate() {
            let mut node_preds = HashSet::with_hasher_in(BuildNoHashHasher::new(), &allocator);

            for resource in node.resources() {
                // If another node accesses the same resource it must
                // run before this node, i.e. become its predecessor.
                // This can be true for many nodes.
                if let Some(preds) = resource_accesses.get(&resource.id) {
                    for pred in preds {
                        node_preds.insert(*pred);
                    }
                }

                // Node::resources should return every resource only once.
                // This means that every if is unique and only inserted
                // once into `accesses`.
                // The implementation of `Node::resources` must guarantee this
                // in order for this function to operate correctly.
                let accesses = resource_accesses
                    .entry(resource.id)
                    .or_insert_with(|| Vec::new_in(&allocator));
                accesses.push(index);
                debug_assert_eq!(accesses.iter().filter(|v| **v == index).count(), 1);
            }

            for succ in &node_preds {
                unsafe {
                    successors.get_unchecked_mut(*succ).push(index);
                }
            }

            if node_preds.is_empty() {
                queue.push(index);
            } else {
                unsafe {
                    *predecessors.get_unchecked_mut(index) = Some(node_preds);
                }
            }
        }

        self.resource_map_cap = resource_accesses.capacity();
        self.predecessors_cap = predecessors.capacity();
        self.sucessors_cap = successors.capacity();

        let mut steps = std::vec::Vec::with_capacity(nodes.len());

        loop {
            // Gather all nodes that have no more predecessors,
            // i.e. all nodes that can be executed now.
            let indices = queue.take_and_advance();

            // Since we have no cycles in predecessors, this loop will always
            // terminate at some point.
            if indices.is_empty() {
                debug_assert!(predecessors.iter().all(|v| v.is_none()));
                break;
            }

            // We batch all barriers required to run all nodes.
            // This allows the caller to insert all barriers
            // using a single call.

            for &index in indices {
                for succ in &successors[index] {
                    if let Some(preds) = &mut predecessors[*succ] {
                        preds.remove(&index);
                        if preds.is_empty() {
                            predecessors[*succ] = None;

                            // Safety:
                            // We have allocated a `Queue` with exactly the number of
                            // nodes to schedule.
                            // Since we remove the node after pushing it no node will
                            // ever get inserted twice.
                            unsafe {
                                queue.push_unchecked(*succ);
                            }
                        }
                    }
                }

                let node = unsafe { nodes.get_unchecked(index) };

                for res in node.resources() {
                    let access = resources.access(res.id);

                    // We can skip a barrier if the resource is already tagged with the
                    // required access flags, but this is only possible for read-only
                    // access since a WRITE->WRITE still requires the previous write to
                    // become visible to prevent WRITE-AFTER-WRITE hazards.
                    if res.access == access && res.access.is_read_only() {
                        continue;
                    }

                    steps.push(Step::Barrier(Barrier {
                        resource: res.id,
                        src_access: access,
                        dst_access: res.access,
                    }));

                    resources.set_access(res.id, res.access);
                }
            }

            for &index in indices {
                let node = unsafe { nodes.get_unchecked(index) };
                steps.push(Step::Node(node));
            }
        }

        steps
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(super) enum Step<N, R> {
    Node(N),
    Barrier(Barrier<R>),
}

impl<N, R> Step<N, R> {
    #[inline]
    pub(super) const fn is_barrier(&self) -> bool {
        matches!(self, Self::Barrier(_))
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(super) struct Barrier<T> {
    pub(super) resource: T,
    pub(super) src_access: AccessFlags,
    pub(super) dst_access: AccessFlags,
}

pub(super) trait ResourceMap {
    type Id;

    fn access(&self, id: Self::Id) -> AccessFlags;

    fn set_access(&mut self, id: Self::Id, access: AccessFlags);
}

pub(super) trait Node {
    type ResourceId: 'static;

    /// Returns every resource that is accessed by this node.
    ///
    /// **Note: This function should only return every resource once in order for [`schedule`] to
    /// operate correctly.**
    fn resources(&self) -> &[Resource<Self::ResourceId>];
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(super) struct Resource<T> {
    pub(super) id: T,
    pub(super) access: AccessFlags,
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use allocator_api2::alloc::Global;

    use crate::api::scheduler::{Barrier, Step};
    use crate::backend::AccessFlags;

    use super::{Node, Resource, ResourceMap, Scheduler};

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct TestNode {
        id: u64,
        resources: Vec<Resource<u64>>,
    }

    impl Node for TestNode {
        type ResourceId = u64;

        fn resources(&self) -> &[Resource<u64>] {
            &self.resources
        }
    }

    impl ResourceMap for &mut HashMap<u64, Resource<u64>> {
        type Id = u64;

        fn access(&self, id: Self::Id) -> AccessFlags {
            self.get(&id).unwrap().access
        }

        fn set_access(&mut self, id: Self::Id, access: AccessFlags) {
            self.get_mut(&id).unwrap().access = access;
        }
    }

    fn schedule<'a>(
        resources: &'a mut HashMap<u64, Resource<u64>>,
        nodes: &'a [TestNode],
    ) -> Vec<Step<&'a TestNode, u64>> {
        let mut scheduler = Scheduler::new();
        scheduler.schedule(resources, Global, nodes)
    }

    #[test]
    fn schedule_simple() {
        // |---|     |---|
        // | 0 | --> | 2 |
        // |---|     |---| -> |---|
        // | 1 | -----------> | 3 |
        // |---|              |---|
        let mut resources = HashMap::from(core::array::from_fn::<_, 2, _>(|index| {
            (
                index as u64,
                Resource {
                    id: index as u64,
                    access: AccessFlags::empty(),
                },
            )
        }));
        let nodes = vec![
            TestNode {
                id: 0,
                resources: vec![Resource {
                    id: 0,
                    access: AccessFlags::TRANSFER_WRITE,
                }],
            },
            TestNode {
                id: 1,
                resources: vec![Resource {
                    id: 1,
                    access: AccessFlags::TRANSFER_WRITE,
                }],
            },
            TestNode {
                id: 2,
                resources: vec![Resource {
                    id: 0,
                    access: AccessFlags::SHADER_READ,
                }],
            },
            TestNode {
                id: 3,
                resources: vec![
                    Resource {
                        id: 0,
                        access: AccessFlags::SHADER_READ,
                    },
                    Resource {
                        id: 1,
                        access: AccessFlags::SHADER_READ,
                    },
                ],
            },
        ];
        let steps: Vec<_> = schedule(&mut resources, &nodes);
        assert_eq!(
            steps,
            [
                Step::Barrier(Barrier {
                    resource: 0,
                    src_access: AccessFlags::empty(),
                    dst_access: AccessFlags::TRANSFER_WRITE
                }),
                Step::Barrier(Barrier {
                    resource: 1,
                    src_access: AccessFlags::empty(),
                    dst_access: AccessFlags::TRANSFER_WRITE
                }),
                Step::Node(&nodes[0]),
                Step::Node(&nodes[1]),
                Step::Barrier(Barrier {
                    resource: 0,
                    src_access: AccessFlags::TRANSFER_WRITE,
                    dst_access: AccessFlags::SHADER_READ,
                }),
                Step::Node(&nodes[2]),
                Step::Barrier(Barrier {
                    resource: 1,
                    src_access: AccessFlags::TRANSFER_WRITE,
                    dst_access: AccessFlags::SHADER_READ,
                }),
                Step::Node(&nodes[3])
            ]
        );
    }

    #[test]
    fn schedule_read_and_write() {
        // |---|     |---|     |---|
        // | 0 | --> | 1 | --> | 2 |
        // |---|     |---|     |---|
        let mut resources = HashMap::from(core::array::from_fn::<_, 2, _>(|index| {
            (
                index as u64,
                Resource {
                    id: index as u64,
                    access: AccessFlags::empty(),
                },
            )
        }));
        let nodes = vec![
            TestNode {
                id: 0,
                resources: vec![Resource {
                    id: 0,
                    access: AccessFlags::TRANSFER_WRITE,
                }],
            },
            TestNode {
                id: 1,
                resources: vec![
                    Resource {
                        id: 0,
                        access: AccessFlags::TRANSFER_READ,
                    },
                    Resource {
                        id: 1,
                        access: AccessFlags::TRANSFER_WRITE,
                    },
                ],
            },
            TestNode {
                id: 2,
                resources: vec![Resource {
                    id: 1,
                    access: AccessFlags::SHADER_READ,
                }],
            },
        ];

        let steps: Vec<_> = schedule(&mut resources, &nodes);
        assert_eq!(
            steps,
            [
                Step::Barrier(Barrier {
                    resource: 0,
                    src_access: AccessFlags::empty(),
                    dst_access: AccessFlags::TRANSFER_WRITE,
                }),
                Step::Node(&nodes[0]),
                Step::Barrier(Barrier {
                    resource: 0,
                    src_access: AccessFlags::TRANSFER_WRITE,
                    dst_access: AccessFlags::TRANSFER_READ,
                }),
                Step::Barrier(Barrier {
                    resource: 1,
                    src_access: AccessFlags::empty(),
                    dst_access: AccessFlags::TRANSFER_WRITE,
                }),
                Step::Node(&nodes[1]),
                Step::Barrier(Barrier {
                    resource: 1,
                    src_access: AccessFlags::TRANSFER_WRITE,
                    dst_access: AccessFlags::SHADER_READ,
                }),
                Step::Node(&nodes[2]),
            ]
        );
    }

    #[test]
    fn schedule_write_after_write() {
        // |---|     |---|     |---|
        // | 0 | --> | 1 | --> | 2 |
        // |---|     |---|     |---|
        let mut resources = HashMap::from([(
            0,
            Resource {
                id: 0,
                access: AccessFlags::empty(),
            },
        )]);
        let nodes = vec![
            TestNode {
                id: 0,
                resources: vec![Resource {
                    id: 0,
                    access: AccessFlags::TRANSFER_WRITE,
                }],
            },
            TestNode {
                id: 1,
                resources: vec![Resource {
                    id: 0,
                    access: AccessFlags::TRANSFER_WRITE,
                }],
            },
            TestNode {
                id: 2,
                resources: vec![Resource {
                    id: 0,
                    access: AccessFlags::TRANSFER_WRITE,
                }],
            },
        ];

        let steps: Vec<_> = schedule(&mut resources, &nodes);
        assert_eq!(
            steps,
            [
                Step::Barrier(Barrier {
                    resource: 0,
                    src_access: AccessFlags::empty(),
                    dst_access: AccessFlags::TRANSFER_WRITE
                }),
                Step::Node(&nodes[0]),
                Step::Barrier(Barrier {
                    resource: 0,
                    src_access: AccessFlags::TRANSFER_WRITE,
                    dst_access: AccessFlags::TRANSFER_WRITE
                }),
                Step::Node(&nodes[1]),
                Step::Barrier(Barrier {
                    resource: 0,
                    src_access: AccessFlags::TRANSFER_WRITE,
                    dst_access: AccessFlags::TRANSFER_WRITE
                }),
                Step::Node(&nodes[2]),
            ]
        );
    }
}
