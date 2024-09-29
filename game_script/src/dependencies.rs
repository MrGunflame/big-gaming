use std::collections::HashMap;
use std::ops::Deref;
use std::sync::Arc;

use game_common::components::components::RawComponent;
use game_common::world::World;
use game_tracing::trace_span;
use game_wasm::entity::EntityId;
use game_wasm::resource::RuntimeResourceId;
use game_wasm::world::RecordReference;

use crate::effect::Effects;
use crate::events::DispatchEvent;
use crate::instance::HostBufferPool;
use crate::{Handle, InvalidHandle, Invocation, Pointer};

pub struct Dependencies {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Dependency {
    HostBuffer(HostBufferDependency),
    Component(ComponentDependency),
    Resource(ResourceDependency),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct HostBufferDependency {
    pub(crate) key: u32,
    pub(crate) buffer: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ComponentDependency {
    pub(crate) entity_id: EntityId,
    pub(crate) component_id: RecordReference,
    pub(crate) component: RawComponent,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ResourceDependency {
    pub(crate) id: RuntimeResourceId,
    pub(crate) data: Arc<[u8]>,
}

pub(crate) struct EffectCache {
    entries: HashMap<(Handle, Pointer), Entry>,
}

impl EffectCache {
    pub fn get(
        &self,
        invocation: &Invocation,
        world: &World,
        host_buffer_pool: &HostBufferPool,
    ) -> Option<&Entry> {
        let _span = trace_span!("EffectCache::get").entered();

        let entry = self.entries.get(&(invocation.script, invocation.fn_ptr))?;
        for dependency in &entry.dependencies {
            match dependency {
                Dependency::HostBuffer(dependency) => {
                    let Some(index) = invocation.host_buffers.get(dependency.key as usize) else {
                        return None;
                    };

                    let buffer = host_buffer_pool.get(*index).unwrap();

                    if buffer != dependency.buffer {
                        return None;
                    }
                }
                Dependency::Component(dependency) => {
                    let Some(component) = world.get(dependency.entity_id, dependency.component_id)
                    else {
                        return None;
                    };

                    if *component != dependency.component {
                        return None;
                    }
                }
                Dependency::Resource(dependency) => {
                    let Some(resource) = world.get_resource(dependency.id) else {
                        return None;
                    };

                    if resource != dependency.data.deref() {
                        return None;
                    }
                }
            }
        }

        Some(entry)
    }

    pub fn insert(&mut self, entry: Entry) {
        let _span = trace_span!("EffectCache::insert").entered();
    }
}

pub(crate) struct Entry {
    pub(crate) dependencies: Vec<Dependency>,
    pub(crate) effects: Effects,
    pub(crate) events: DispatchEvent,
}

impl Entry {
    fn remove_locals(&mut self) {
        for dependency in &self.dependencies {}
    }
}
