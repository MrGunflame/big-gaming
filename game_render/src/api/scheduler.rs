use std::collections::HashMap;
use std::hash::Hash;

use crate::backend::{AccessFlags, BarrierPipelineStage};

pub(super) fn schedule<'a, T, M>(
    resources: &mut M,
    nodes: &'a [T],
) -> Vec<Step<&'a T, T::ResourceId>>
where
    T: Node<M>,
    T::ResourceId: Copy + Hash + Eq,
    M: ResourceMap<Id = T::ResourceId>,
{
    let mut resource_accesses = HashMap::<_, Vec<_>>::new();
    let mut predecessors = HashMap::<_, Vec<_>>::new();

    for (index, node) in nodes.iter().enumerate() {
        let mut node_preds = Vec::new();

        for resource in node.resources(&resources) {
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
            let accesses = resource_accesses.entry(resource.id).or_default();
            accesses.push(index);
            debug_assert_eq!(accesses.iter().filter(|v| **v == index).count(), 1);
        }

        predecessors.insert(index, node_preds);
    }

    let mut steps = Vec::new();
    loop {
        // Gather all nodes that have no more predecessors,
        // i.e. all nodes that can be executed now.
        let mut indices: Vec<_> = predecessors
            .iter_mut()
            .filter_map(|(index, preds)| preds.is_empty().then_some(*index))
            .collect();

        // Since we have no cycles in predecessors, this loop will always
        // terminate at some point.
        if indices.is_empty() {
            debug_assert!(predecessors.is_empty());
            break;
        }

        // We should keep the order of nodes if possible.
        indices.sort();

        // We batch all barriers required to run all nodes.
        // This allows the caller to insert all barriers
        // using a single call.

        for &index in &indices {
            predecessors.remove(&index);
            for preds in predecessors.values_mut() {
                preds.retain(|pred| *pred != index);
            }

            let node = &nodes[index];
            for res in node.resources(&resources) {
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

        for index in &indices {
            steps.push(Step::Node(&nodes[*index]));
        }
    }

    steps
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(super) enum Step<N, R> {
    Node(N),
    Barrier(Barrier<R>),
}

impl<N, R> Step<N, R> {
    #[inline]
    pub const fn is_node(&self) -> bool {
        matches!(self, Self::Node(_))
    }

    #[inline]
    pub const fn is_barrier(&self) -> bool {
        matches!(self, Self::Barrier(_))
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(super) struct Barrier<T> {
    pub(super) resource: T,
    pub(super) src_access: AccessFlags,
    pub(super) dst_access: AccessFlags,
}

pub trait ResourceMap {
    type Id;

    fn access(&self, id: Self::Id) -> AccessFlags;

    fn set_access(&mut self, id: Self::Id, access: AccessFlags);
}

pub(super) trait Node<M> {
    type ResourceId;

    /// Returns every resource that is accessed by this node.
    ///
    /// **Note: This function should only return every resource once in order for [`schedule`] to
    /// operate correctly.**
    fn resources(&self, resources: &M) -> Vec<Resource<Self::ResourceId>>;
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(super) struct Resource<T> {
    pub(super) id: T,
    pub(super) access: AccessFlags,
}

#[derive(Copy, Clone, Debug)]
struct Edge {
    /// The access that the sucessors needs from its predecessor.
    access: AccessFlags,
    /// The stage in which the access occurs.
    stage: BarrierPipelineStage,
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::api::scheduler::{Barrier, Step};
    use crate::backend::AccessFlags;

    use super::{schedule, Node, Resource, ResourceMap};

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct TestNode {
        id: u64,
        resources: Vec<Resource<u64>>,
    }

    impl Node<HashMap<u64, Resource<u64>>> for TestNode {
        type ResourceId = u64;

        fn resources(
            &self,
            _resources: &HashMap<u64, Resource<u64>>,
        ) -> Vec<Resource<Self::ResourceId>> {
            self.resources.clone()
        }
    }

    impl ResourceMap for HashMap<u64, Resource<u64>> {
        type Id = u64;

        fn access(&self, id: Self::Id) -> AccessFlags {
            self.get(&id).unwrap().access
        }

        fn set_access(&mut self, id: Self::Id, access: AccessFlags) {
            self.get_mut(&id).unwrap().access = access;
        }
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
