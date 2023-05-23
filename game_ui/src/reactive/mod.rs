use std::collections::{HashMap, VecDeque};
use std::ops::Deref;
use std::sync::Arc;

use bevy_ecs::prelude::Component;
use bevy_ecs::world::World;
use parking_lot::{Mutex, MutexGuard};
use slotmap::{DefaultKey, SlotMap};

use crate::events::Events;
use crate::render::layout::{Key, LayoutTree};

use self::effect::{Effect, EffectId};
use self::signal::{Signal, SignalId};

mod effect;
mod node;
mod signal;

pub use effect::create_effect;
pub use node::Node;
pub use signal::create_signal;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct NodeId(DefaultKey);

#[derive(Clone)]
pub struct Scope {
    document: Document,
    id: Option<NodeId>,
    // Ref to parent, or none if root.
    parent: Option<NodeId>,
}

impl Scope {
    // pub fn child(&self, child: NodeId) -> Self {
    //     let mut doc = self.document.inner.lock();
    //     let parent = doc.parents.get(&child).copied().unwrap();

    //     Self {
    //         document: self.document.clone(),
    //         id: Some(child),
    //         parent,
    //     }
    // }

    pub fn id(&self) -> Option<NodeId> {
        self.id
    }

    pub fn push(&self, node: Node) -> Scope {
        dbg!(self.id, self.parent);

        let mut doc = self.document.inner.lock();

        let id = doc.nodes.insert(NodeStore::default());
        doc.queue.push_back(Event::PushNode(NodeId(id), node));

        doc.parents.insert(NodeId(id), self.id);
        if let Some(parent) = self.parent {
            doc.children.entry(parent).or_default().push(NodeId(id));
        }

        Scope {
            document: self.document.clone(),
            id: Some(NodeId(id)),
            parent: self.id,
        }
    }

    pub fn remove(&self, id: NodeId) {
        let mut doc = self.document.inner.lock();

        doc.queue.push_back(Event::RemoveNode(id));
    }

    // /// Update in place
    // pub fn update(&self, id: NodeId) -> &mut Node {
    //     let mut doc = self.document.inner.lock();
    // }
}

// struct NodeMut<'a> {
//     inner: MutexGuard<'a, DocumentInner>,
//     id: NodeId,
// }

// impl<'a> Deref for NodeMut<'a> {
//     type Target = Node;

//     fn deref(&self) -> &Self::Target {
//         self.inner.nodes.get(self.id.0).unwrap()
//     }
// }

#[derive(Clone, Default, Component)]
pub struct Document {
    inner: Arc<Mutex<DocumentInner>>,
    signal_stack: Arc<Mutex<Vec<SignalId>>>,
}

#[derive(Default)]
struct DocumentInner {
    // EffectId
    effects: SlotMap<DefaultKey, Effect>,
    // SignalId
    signals: SlotMap<DefaultKey, Signal>,

    // SignalId => vec![EffectId]
    signal_effects: HashMap<DefaultKey, Vec<DefaultKey>>,

    // Backlogged queued effects.
    effect_queue: Vec<EffectId>,

    pub nodes: SlotMap<DefaultKey, NodeStore>,
    // parent => vec![child]
    pub children: HashMap<NodeId, Vec<NodeId>>,
    // child => parent, none if parent is root
    pub parents: HashMap<NodeId, Option<NodeId>>,
    // SignalId => NodeId
    signal_targets: HashMap<DefaultKey, Option<DefaultKey>>,

    queue: VecDeque<Event>,

    node_mappings: HashMap<NodeId, Key>,
}

impl Document {
    pub fn new<F>(f: F) -> Self
    where
        F: Fn(Scope) + Send + Sync + 'static,
    {
        let this = Self::default();
        let cx = this.root_scope();
        f(cx);
        this
    }

    pub fn root_scope(&self) -> Scope {
        let mut doc = self.inner.lock();
        let id = doc.nodes.insert(NodeStore::default());

        Scope {
            document: self.clone(),
            id: None,
            parent: None,
        }
    }

    pub fn drive(&self, world: &World, tree: &mut LayoutTree, events: &mut Events) {
        let mut doc = self.inner.lock();

        doc.effect_queue.dedup();
        let queue = doc.effect_queue.clone();
        drop(doc);
        for effect_id in queue {
            tracing::trace!("call Effect({:?})", effect_id);

            let mut doc = self.inner.lock();
            let effect = doc.effects.get_mut(effect_id.0).unwrap();

            if effect.first_run {
                effect.first_run = false;
                let effect = effect.clone();
                drop(doc);

                (effect.f)(world);

                let stack = std::mem::take(&mut *self.signal_stack.lock());
                tracing::trace!("subscribing Effect({:?}) to signals {:?}", effect_id, stack);
                let mut doc = self.inner.lock();

                for signal in stack {
                    doc.signal_effects
                        .entry(signal.0)
                        .or_default()
                        .push(effect_id.0);
                }

                drop(doc);
            } else {
                let effect = effect.clone();
                drop(doc);
                (effect.f)(world);
            }
        }

        let mut doc = self.inner.lock();
        doc.effect_queue.clear();

        while let Some(event) = doc.queue.pop_front() {
            match event {
                Event::PushNode(id, node) => {
                    tracing::trace!("spawn node {:?}", id);

                    let parent = doc
                        .parents
                        .get(&id)
                        .map(|p| p.map(|p| doc.node_mappings.get(&p).copied()))
                        .flatten()
                        .flatten();

                    dbg!(&id);
                    dbg!(&doc.parents);
                    dbg!(&node.element);
                    dbg!(parent);

                    let key = tree.push(parent, node.element);

                    doc.node_mappings.insert(id, key);
                    events.insert(key, node.events);
                }
                Event::RemoveNode(id) => {
                    tracing::trace!("despawn node {:?}", id);

                    let key = doc.node_mappings.remove(&id).unwrap();
                    tree.remove(key);
                    events.remove(key);
                }
            }
        }
    }
}

#[derive(Clone, Default)]
pub struct NodeStore {
    // Effects registered on this node.
    effects: Vec<Effect>,
    // Signals registred on this node.
    signals: Vec<DefaultKey>,
}

pub enum Event {
    PushNode(NodeId, Node),
    RemoveNode(NodeId),
}
