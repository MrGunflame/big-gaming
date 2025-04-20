use std::hash::Hash;

use allocator_api2::alloc::Allocator;
use allocator_api2::vec::Vec;
use game_tracing::trace_span;
use hashbrown::HashMap;
use nohash_hasher::BuildNoHashHasher;

use crate::backend::AccessFlags;

type UsizeMap<T, A> = HashMap<usize, T, BuildNoHashHasher<usize>, A>;

#[derive(Debug)]
pub struct Scheduler<M, A> {
    pub resources: M,
    pub allocator: A,
}

impl<M, A> Scheduler<M, A>
where
    A: Allocator,
{
    pub fn schedule<'a, T>(&mut self, nodes: &'a [T]) -> std::vec::Vec<Step<&'a T, T::ResourceId>>
    where
        T: Node,
        T::ResourceId: Copy + Hash + Eq,
        M: ResourceMap<Id = T::ResourceId>,
    {
        let _span = trace_span!("Scheduler::schedule").entered();

        let mut resource_accesses =
            HashMap::<_, Vec<_, &A>, _, &A>::with_capacity_in(nodes.len(), &self.allocator);
        // We use linear indices as the key, they are already uniformly
        // distributed so we can skip the hashing.
        let mut predecessors = UsizeMap::<Vec<_, &A>, _>::with_capacity_and_hasher_in(
            nodes.len(),
            BuildNoHashHasher::new(),
            &self.allocator,
        );
        let mut successors = UsizeMap::<Vec<_, &A>, _>::with_capacity_and_hasher_in(
            nodes.len(),
            BuildNoHashHasher::new(),
            &self.allocator,
        );

        let mut queue = Vec::with_capacity_in(nodes.len(), &self.allocator);

        for (index, node) in nodes.iter().enumerate() {
            let mut node_preds = Vec::new_in(&self.allocator);

            for resource in node.resources() {
                // If another node accesses the same resource it must
                // run before this node, i.e. become its predecessor.
                // This can be true for many nodes.
                if let Some(preds) = resource_accesses.get(&resource.id) {
                    for pred in preds {
                        node_preds.push(*pred);
                    }
                }

                // Node::resources should return every resource only once.
                // This means that every if is unique and only inserted
                // once into `accesses`.
                // The implementation of `Node::resources` must guarantee this
                // in order for this function to operate correctly.
                let accesses = resource_accesses
                    .entry(resource.id)
                    .or_insert_with(|| Vec::new_in(&self.allocator));
                accesses.push(index);
                debug_assert_eq!(accesses.iter().filter(|v| **v == index).count(), 1);
            }

            for succ in &node_preds {
                successors
                    .entry(*succ)
                    .or_insert_with(|| Vec::new_in(&self.allocator))
                    .push(index);
            }

            if node_preds.is_empty() {
                queue.push(index);
            } else {
                predecessors.insert(index, node_preds);
            }
        }

        let mut steps = std::vec::Vec::with_capacity(nodes.len());
        loop {
            // Gather all nodes that have no more predecessors,
            // i.e. all nodes that can be executed now.
            let mut indices = Vec::with_capacity_in(queue.len(), &self.allocator);
            indices.extend(queue.drain(..));

            // Since we have no cycles in predecessors, this loop will always
            // terminate at some point.
            if indices.is_empty() {
                debug_assert!(predecessors.is_empty());
                break;
            }

            // We should keep the order of nodes if possible.
            // The `VecMap` iterator already guarantees that elements are in order
            // of their index.
            debug_assert!(indices.is_sorted());

            // We batch all barriers required to run all nodes.
            // This allows the caller to insert all barriers
            // using a single call.

            for &index in &indices {
                if let Some(succs) = successors.get(&index) {
                    for succ in succs {
                        if let Some(preds) = predecessors.get_mut(succ) {
                            preds.retain(|pred| *pred != index);
                            if preds.is_empty() {
                                predecessors.remove(succ);
                                queue.push(*succ);
                            }
                        }
                    }
                }

                let node = &nodes[index];
                for res in node.resources() {
                    let access = self.resources.access(res.id);

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

                    self.resources.set_access(res.id, res.access);
                }
            }

            // The above is possible to get fucked up like so:
            // 0 -> 5
            // 1 -> 2
            // FIXME: Time to redo this entire thing.
            queue.sort();

            for index in &indices {
                steps.push(Step::Node(&nodes[*index]));
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
        let mut scheduler = Scheduler {
            resources,
            allocator: Global,
        };
        scheduler.schedule(nodes)
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
