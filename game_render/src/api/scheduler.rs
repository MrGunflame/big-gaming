use std::collections::HashMap;
use std::hash::Hash;
use std::marker::PhantomData;

use crate::backend::{AccessFlags, BarrierPipelineStage};

pub struct GraphScheduler<T> {
    t: PhantomData<T>,
}

impl<T> GraphScheduler<T>
where
    T: Node + std::fmt::Debug,
    T::Id: Copy + Hash + Eq + std::fmt::Debug,
{
    pub fn new() -> Self {
        Self { t: PhantomData }
    }

    pub fn schedule<'a>(
        &mut self,
        resources: &mut HashMap<T::Id, Resource<u64>>,
        nodes: &'a [T],
    ) -> Vec<Step<&'a T, u64>> {
        let mut resource_accesses = HashMap::<_, Vec<_>>::new();
        let mut predecessors = HashMap::<_, Vec<_>>::new();

        for (index, node) in nodes.iter().enumerate() {
            let mut node_preds = Vec::new();

            for resource in node.resources() {
                if let Some(preds) = resource_accesses.get(&resource.id) {
                    for pred in preds {
                        node_preds.push(*pred);
                    }
                }

                resource_accesses
                    .entry(resource.id)
                    .or_default()
                    .push(index);
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
                for res in node.resources() {
                    let resource = resources.get_mut(&res.id).unwrap();

                    if res.access == resource.access {
                        continue;
                    }

                    steps.push(Step::Barrier(Barrier {
                        resource: resource.id,
                        src_access: resource.access,
                        dst_access: res.access,
                    }));

                    resource.access = res.access;
                }
            }

            for index in &indices {
                steps.push(Step::Node(&nodes[*index]));
            }
        }

        steps
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Step<N, R> {
    Node(N),
    Barrier(Barrier<R>),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct Barrier<T> {
    resource: T,
    src_access: AccessFlags,
    dst_access: AccessFlags,
}

pub(crate) trait Node {
    type Id;

    fn id(&self) -> Self::Id;

    fn resources(&self) -> Vec<Resource<Self::Id>>;
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Resource<T> {
    id: T,
    access: AccessFlags,
    stage: BarrierPipelineStage,
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
    use crate::backend::{AccessFlags, BarrierPipelineStage};

    use super::{GraphScheduler, Node, Resource};

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct TestNode {
        id: u64,
        resources: Vec<Resource<u64>>,
    }

    impl Node for TestNode {
        type Id = u64;

        fn id(&self) -> Self::Id {
            self.id
        }

        fn resources(&self) -> Vec<Resource<Self::Id>> {
            self.resources.clone()
        }
    }

    #[test]
    fn scheduler_single_frame() {
        let mut scheduler = GraphScheduler::new();

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
                    stage: BarrierPipelineStage::Top,
                },
            )
        }));
        let nodes = vec![
            TestNode {
                id: 0,
                resources: vec![Resource {
                    id: 0,
                    access: AccessFlags::TRANSFER_WRITE,
                    stage: BarrierPipelineStage::Transfer,
                }],
            },
            TestNode {
                id: 1,
                resources: vec![Resource {
                    id: 1,
                    access: AccessFlags::TRANSFER_WRITE,
                    stage: BarrierPipelineStage::Transfer,
                }],
            },
            TestNode {
                id: 2,
                resources: vec![Resource {
                    id: 0,
                    access: AccessFlags::SHADER_READ,
                    stage: BarrierPipelineStage::Top,
                }],
            },
            TestNode {
                id: 3,
                resources: vec![
                    Resource {
                        id: 0,
                        access: AccessFlags::SHADER_READ,
                        stage: BarrierPipelineStage::Top,
                    },
                    Resource {
                        id: 1,
                        access: AccessFlags::SHADER_READ,
                        stage: BarrierPipelineStage::Top,
                    },
                ],
            },
        ];
        let steps: Vec<_> = scheduler.schedule(&mut resources, &nodes);
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
}
