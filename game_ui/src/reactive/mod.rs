use std::collections::HashMap;
use std::sync::Arc;

use bevy_ecs::prelude::Component;
use bevy_ecs::world::World;
use parking_lot::Mutex;
use slotmap::{DefaultKey, SlotMap};

use self::effect::Effect;
use self::signal::Signal;

mod effect;
mod signal;
mod view;

pub use effect::create_effect;
pub use signal::create_signal;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct NodeId(DefaultKey);

#[derive(Clone)]
pub struct Scope {
    document: Document,
    id: NodeId,
    parent: Option<NodeId>,
}

#[derive(Clone, Default, Component)]
pub struct Document {
    inner: Arc<Mutex<DocumentInner>>,
}

#[derive(Default)]
struct DocumentInner {
    pub nodes: SlotMap<DefaultKey, Node>,
    // parent => vec![child]
    pub children: HashMap<NodeId, Vec<NodeId>>,
    // child => parent
    pub parents: HashMap<NodeId, NodeId>,
    signals: SlotMap<DefaultKey, Signal>,
    // SignalId => NodeId
    signal_targets: HashMap<DefaultKey, DefaultKey>,
    signal_queue: Vec<DefaultKey>,
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
        let id = doc.nodes.insert(Node::default());

        Scope {
            document: self.clone(),
            id: NodeId(id),
            parent: None,
        }
    }

    pub fn drive(&self, world: &World) {
        let mut doc = self.inner.lock();

        while let Some(signal_id) = doc.signal_queue.pop() {
            let node_id = doc.signal_targets.get(&signal_id).unwrap();
            let node = doc.nodes.get(*node_id).unwrap();

            for effect in &node.effects {
                (effect.f)(world);
            }
        }
    }
}

#[derive(Clone, Default)]
pub struct Node {
    // Effects registered on this node.
    effects: Vec<Effect>,
    // Signals registred on this node.
    signals: Vec<DefaultKey>,
}
