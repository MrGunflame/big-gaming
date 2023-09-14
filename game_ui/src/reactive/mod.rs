use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

use parking_lot::Mutex;
use slotmap::{new_key_type, SlotMap};

use crate::events::Events;
use crate::layout::{Key, LayoutTree};
use crate::style::Style;
use crate::widgets::Widget;

use self::effect::{Effect, EffectId};
use self::signal::SignalId;

mod effect;
mod node;
mod signal;

pub use node::Node;
pub use signal::{ReadSignal, WriteSignal};

thread_local! {
    static ACTIVE_EFFECT: RefCell<ActiveEffect> = RefCell::new(ActiveEffect {
        first_run: false,
        stack: Vec::new(),
    });
}

#[derive(Clone, Debug)]
struct ActiveEffect {
    first_run: bool,
    stack: Vec<SignalId>,
}

new_key_type! {
    pub struct NodeId;
}

#[derive(Debug, Clone)]
pub struct Scope {
    document: Document,
    // Ref to parent, or none if root.
    id: Option<NodeId>,
}

impl Scope {
    pub fn append<T>(&self, widget: T) -> Scope
    where
        T: Widget,
    {
        widget.build(self)
    }

    pub fn id(&self) -> Option<NodeId> {
        self.id
    }

    pub fn push(&self, node: Node) -> Scope {
        let mut doc = self.document.inner.lock();

        let id = doc.nodes.push(self.id);
        doc.events.push_back(Event::CreateNode(id, node));

        Scope {
            document: self.document.clone(),
            id: Some(id),
        }
    }

    pub fn remove(&self, id: NodeId) {
        let mut doc = self.document.inner.lock();
        doc.events.push_back(Event::RemoveNode(id));
    }

    /// Update in place
    pub fn update(&self, id: NodeId, node: Node) {
        let mut doc = self.document.inner.lock();
        doc.events.push_back(Event::UpdateNode(id, node));
    }

    pub fn set_style(&self, id: NodeId, style: Style) {
        let mut doc = self.document.inner.lock();
        doc.events.push_back(Event::UpdateStyle(id, style));
    }
}

#[derive(Clone, Debug, Default)]
pub struct Runtime {
    inner: Arc<Mutex<RuntimeInner>>,
}

impl Runtime {
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Debug, Default)]
struct RuntimeInner {
    effects: SlotMap<EffectId, Effect>,
    next_signal_id: u64,

    /// Effects scheduled for execution.
    queue: HashSet<EffectId>,
    /// What effects are subscribed to signals.
    subscribers: HashMap<SignalId, Vec<EffectId>>,
    subscribers_by_effect: HashMap<EffectId, Vec<SignalId>>,
}

// Note that `Document` has no `Default` impl to prevent accidental
// creation on a new `Runtime` (which has a `Default` impl).
#[derive(Clone, Debug)]
pub struct Document {
    runtime: Runtime,
    inner: Arc<Mutex<DocumentInner>>,
}

#[derive(Debug, Default)]
struct DocumentInner {
    nodes: NodeHierarchy,
    events: VecDeque<Event>,

    effects: HashSet<EffectId>,
    effects_by_node: HashMap<Option<NodeId>, Vec<EffectId>>,
}

impl Document {
    pub fn new(runtime: Runtime) -> Self {
        Self {
            runtime,
            inner: Arc::default(),
        }
    }

    pub fn root_scope(&self) -> Scope {
        Scope {
            document: self.clone(),
            id: None,
        }
    }

    pub fn runtime(&self) -> &Runtime {
        &self.runtime
    }

    pub fn run_effects(&self) {
        let mut doc = self.inner.lock();

        let mut rt = self.runtime.inner.lock();

        let queue = rt.queue.clone();

        for effect_id in queue {
            if !doc.effects.contains(&effect_id) {
                continue;
            }

            tracing::trace!("call Effect({:?})", effect_id);

            if cfg!(debug_assertions) {
                let effect = rt.effects.get(effect_id).unwrap();

                tracing::trace!("Calling Effect {:?}", effect);
            }

            let mut effect = rt.effects.get_mut(effect_id).unwrap().clone();

            // Drop the document so that effect callee has full access
            // to the document.
            drop(rt);
            drop(doc);

            if effect.first_run {
                effect.first_run = false;

                ACTIVE_EFFECT.with(|cell| {
                    let mut data = cell.borrow_mut();
                    data.first_run = true;
                });

                (effect.f)();

                let mut stack = ACTIVE_EFFECT.with(|cell| {
                    let mut data = cell.borrow_mut();
                    data.first_run = false;
                    std::mem::take(&mut data.stack)
                });
                tracing::trace!("subscribing Effect({:?}) to signals {:?}", effect_id, stack);

                // We only want to track each effect once.
                stack.dedup();

                rt = self.runtime.inner.lock();
                for signal in stack {
                    rt.subscribers.entry(signal).or_default().push(effect_id);
                    rt.subscribers_by_effect
                        .entry(effect_id)
                        .or_default()
                        .push(signal);
                }
            } else {
                // `first_run` is set to `false` at the end of a first effect
                // call.
                if cfg!(debug_assertions) {
                    ACTIVE_EFFECT.with(|cell| {
                        let data = cell.borrow();
                        assert!(!data.first_run);
                    });
                }

                (effect.f)();

                rt = self.runtime.inner.lock();
            }

            doc = self.inner.lock();

            rt.queue.remove(&effect_id);
        }
    }

    pub fn flush_node_queue(&self, tree: &mut LayoutTree, events: &mut Events) {
        let mut doc = self.inner.lock();
        let mut rt = self.runtime.inner.lock();

        while let Some(event) = doc.events.pop_front() {
            match event {
                Event::CreateNode(id, node) => {
                    tracing::trace!("spawn node {:?} {:?}", id, node);

                    let parent = doc
                        .nodes
                        .parent(id)
                        .map(|parent| doc.nodes.get(parent))
                        .flatten();

                    let key = tree.push(parent, node.element);

                    doc.nodes.set(id, key);
                    events.insert(key, node.events);
                }
                Event::RemoveNode(id) => {
                    tracing::trace!("despawn node {:?}", id);

                    // Reborrow fields so we can move it to closure partially.
                    let doc = &mut *doc;
                    let nodes = &mut doc.nodes;
                    let effects_by_node = &mut doc.effects_by_node;
                    let effects = &mut doc.effects;

                    nodes.remove(id, |node_id, key| {
                        if let Some(key) = key {
                            tree.remove(key);
                            events.remove(key);
                        }

                        if let Some(effect_ids) = effects_by_node.remove(&Some(node_id)) {
                            for id in effect_ids {
                                effects.remove(&id);
                                rt.effects.remove(id);

                                if let Some(signals) = rt.subscribers_by_effect.remove(&id) {
                                    for signal in signals {
                                        rt.subscribers.remove(&signal);
                                    }
                                }
                            }
                        }
                    });
                }
                Event::UpdateNode(id, node) => {
                    tracing::trace!("replace node {:?}", id);

                    if let Some(key) = doc.nodes.get(id) {
                        tree.replace(key, node.element);
                        *events.get_mut(key).unwrap() = node.events;
                    } else {
                        tracing::trace!("node {:?} does not exist", id);
                    }
                }
                Event::UpdateStyle(id, style) => {
                    tracing::trace!("update style {:?}", id);

                    if let Some(key) = doc.nodes.get(id) {
                        tree.get_mut(key).unwrap().style = style;
                    } else {
                        tracing::warn!("node {:?} does not exist", id);
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
pub enum Event {
    CreateNode(NodeId, Node),
    UpdateNode(NodeId, Node),
    RemoveNode(NodeId),
    UpdateStyle(NodeId, Style),
}

#[derive(Clone, Debug, Default)]
struct NodeHierarchy {
    nodes: SlotMap<NodeId, Option<Key>>,
    children: HashMap<NodeId, Vec<NodeId>>,
    parents: HashMap<NodeId, NodeId>,
}

impl NodeHierarchy {
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn push(&mut self, parent: Option<NodeId>) -> NodeId {
        let key = self.nodes.insert(None);

        if let Some(parent) = parent {
            debug_assert!(self.nodes.contains_key(parent));

            self.parents.insert(key, parent);
            self.children.entry(parent).or_default().push(key);
        }

        key
    }

    pub fn remove<F: FnMut(NodeId, Option<Key>)>(&mut self, key: NodeId, mut op: F) {
        let mut queue: VecDeque<_> = [key].into();

        while let Some(key) = queue.pop_front() {
            let k = self.nodes.remove(key).flatten();

            op(key, k);

            if let Some(parent) = self.parents.remove(&key) {
                if let Some(children) = self.children.get_mut(&parent) {
                    children.retain(|id| *id != key);
                }
            }

            if let Some(children) = self.children.remove(&key) {
                queue.extend(children);
            }
        }
    }

    pub fn get(&self, key: NodeId) -> Option<Key> {
        self.nodes.get(key).copied().flatten()
    }

    pub fn set(&mut self, id: NodeId, key: Key) {
        *self.nodes.get_mut(id).unwrap() = Some(key);
    }

    pub fn parent(&self, key: NodeId) -> Option<NodeId> {
        self.parents.get(&key).copied()
    }
}

#[cfg(test)]
mod tests {
    use crate::events::{ElementEventHandlers, Events};
    use crate::layout::LayoutTree;
    use crate::reactive::Runtime;
    use crate::render::{Element, ElementBody};
    use crate::style::Style;

    use super::{Document, Node};

    pub(super) fn create_node() -> Node {
        Node {
            element: Element {
                body: ElementBody::Container,
                style: Style::default(),
            },
            events: ElementEventHandlers::default(),
        }
    }

    #[test]
    fn document_cleanup() {
        let rt = Runtime::new();
        let doc = Document::new(rt);
        let cx = doc.root_scope();

        let mut tree = LayoutTree::new();
        let mut events = Events::new();

        let id = cx.push(create_node()).id().unwrap();

        doc.run_effects();
        doc.flush_node_queue(&mut tree, &mut events);

        cx.remove(id);

        doc.run_effects();
        doc.flush_node_queue(&mut tree, &mut events);

        assert!(doc.inner.lock().nodes.is_empty());
        assert!(tree.is_empty());
        assert!(events.is_empty());
    }

    #[test]
    fn document_cleanup_children() {
        let rt = Runtime::new();
        let doc = Document::new(rt);
        let cx = doc.root_scope();

        let mut tree = LayoutTree::new();
        let mut events = Events::new();

        let id = {
            let cx = cx.push(create_node());
            cx.push(create_node());
            cx.push(create_node());
            cx.id().unwrap()
        };

        doc.run_effects();
        doc.flush_node_queue(&mut tree, &mut events);

        cx.remove(id);

        doc.run_effects();
        doc.flush_node_queue(&mut tree, &mut events);

        assert!(doc.inner.lock().nodes.is_empty());
        assert!(tree.is_empty());
        assert!(events.is_empty());
    }

    #[test]
    fn document_remove_parent_children() {
        let rt = Runtime::new();
        let doc = Document::new(rt);
        let cx = doc.root_scope();

        let mut tree = LayoutTree::new();
        let mut events = Events::new();

        let parent = cx.push(create_node());
        let children = parent.push(create_node());

        doc.run_effects();
        doc.flush_node_queue(&mut tree, &mut events);

        cx.remove(parent.id().unwrap());
        cx.remove(children.id().unwrap());
    }

    #[test]
    fn document_insert_remove() {
        let rt = Runtime::new();
        let doc = Document::new(rt);
        let cx = doc.root_scope();

        let mut tree = LayoutTree::new();
        let mut events = Events::new();

        let node = cx.push(create_node());
        node.remove(node.id().unwrap());

        doc.run_effects();
        doc.flush_node_queue(&mut tree, &mut events);
    }
}
