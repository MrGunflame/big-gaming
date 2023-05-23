use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use bevy_ecs::prelude::Component;
use bevy_ecs::world::World;
use parking_lot::Mutex;
use slotmap::{DefaultKey, SlotMap};

use crate::events::Events;
use crate::render::layout::{Key, LayoutTree};

use self::effect::Effect;
use self::signal::Signal;

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
}

#[derive(Clone, Default, Component)]
pub struct Document {
    inner: Arc<Mutex<DocumentInner>>,
}

#[derive(Default)]
struct DocumentInner {
    pub nodes: SlotMap<DefaultKey, NodeStore>,
    // parent => vec![child]
    pub children: HashMap<NodeId, Vec<NodeId>>,
    // child => parent, none if parent is root
    pub parents: HashMap<NodeId, Option<NodeId>>,
    signals: SlotMap<DefaultKey, Signal>,
    // SignalId => NodeId
    signal_targets: HashMap<DefaultKey, DefaultKey>,
    signal_queue: Vec<DefaultKey>,

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

        // Rerun effects
        while let Some(signal_id) = doc.signal_queue.pop() {
            let node_id = doc.signal_targets.get(&signal_id).unwrap();
            let node = doc.nodes.get(*node_id).unwrap();

            for effect in &node.effects {
                (effect.f)(world);
            }
        }

        while let Some(event) = doc.queue.pop_front() {
            match event {
                Event::PushNode(id, node) => {
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
