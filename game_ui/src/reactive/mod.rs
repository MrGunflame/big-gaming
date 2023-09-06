use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

use parking_lot::Mutex;
use slotmap::{new_key_type, SlotMap};

use crate::events::Events;
use crate::render::layout::{Key, LayoutTree};
use crate::style::Style;
use crate::widgets::Widget;

use self::effect::{Effect, EffectId};
use self::signal::{Signal, SignalId};

mod effect;
mod node;
mod signal;

pub use effect::create_effect;
pub use node::Node;
pub use signal::{create_signal, ReadSignal, WriteSignal};

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
    // pub fn child(&self, child: NodeId) -> Self {
    //     let mut doc = self.document.inner.lock();
    //     let parent = doc.parents.get(&child).copied().unwrap();

    //     Self {
    //         document: self.document.clone(),
    //         id: Some(child),
    //         parent,
    //     }
    // }

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

        let id = doc.nodes.insert(());
        doc.queue.push_back(Event::PushNode(id, node));

        doc.parents.insert(id, self.id);
        if let Some(parent) = self.id {
            doc.children.entry(parent).or_default().push(id);
        }

        Scope {
            document: self.document.clone(),
            id: Some(id),
        }
    }

    pub fn remove(&self, id: NodeId) {
        let mut doc = self.document.inner.lock();

        doc.queue.push_back(Event::RemoveNode(id));
    }

    /// Update in place
    pub fn update(&self, id: NodeId, node: Node) {
        let mut doc = self.document.inner.lock();
        doc.queue.push_back(Event::UpdateNode(id, node));
    }

    pub fn set_style(&self, id: NodeId, style: Style) {
        let mut doc = self.document.inner.lock();
        doc.queue.push_back(Event::UpdateStyle(id, style));
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
    // EffectId
    effects: SlotMap<EffectId, Effect>,
    // SignalId
    signals: SlotMap<SignalId, Signal>,
    // Backlogged queued effects.
    effect_queue: HashSet<EffectId>,

    // SignalId => vec![EffectId]
    signal_effects: HashMap<SignalId, Vec<EffectId>>,
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
    // Effects in this document
    effects: HashSet<EffectId>,

    pub nodes: SlotMap<NodeId, ()>,
    // parent => vec![child]
    pub children: HashMap<NodeId, Vec<NodeId>>,
    // child => parent, none if parent is root
    pub parents: HashMap<NodeId, Option<NodeId>>,
    // SignalId => NodeId
    signal_targets: HashMap<SignalId, Option<NodeId>>,

    queue: VecDeque<Event>,

    node_mappings: HashMap<NodeId, Key>,
}

impl Document {
    pub fn new(runtime: Runtime) -> Self {
        Self {
            runtime,
            inner: Arc::default(),
        }
    }

    pub fn len(&self) -> usize {
        let inner = self.inner.lock();

        let len = inner.nodes.len();

        // Extra assertions for cleanup tests.
        if cfg!(debug_assertions) && len == 0 {
            assert_eq!(inner.children.len(), 0);
            assert_eq!(inner.parents.len(), 0);
            assert_eq!(inner.node_mappings.len(), 0);
        }

        len
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
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

        let queue = rt.effect_queue.clone();

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
                    rt.signal_effects.entry(signal).or_default().push(effect_id);
                }

                *rt.effects.get_mut(effect_id).unwrap() = effect;
            } else {
                // `first_run` is set to `false` at the end of a first effect
                // call.
                if cfg!(debug_assertions) {
                    ACTIVE_EFFECT.with(|cell| {
                        let data = cell.borrow();
                        assert!(!data.first_run);
                    });
                }

                let effect = effect.clone();
                (effect.f)();

                rt = self.runtime.inner.lock();
            }

            doc = self.inner.lock();

            rt.effect_queue.remove(&effect_id);
        }
    }

    pub fn flush_node_queue(&self, tree: &mut LayoutTree, events: &mut Events) {
        let mut doc = self.inner.lock();
        let mut rt = self.runtime.inner.lock();

        'out: while let Some(event) = doc.queue.pop_front() {
            match event {
                Event::PushNode(id, node) => {
                    tracing::trace!("spawn node {:?} {:?}", id, node);

                    let parent = doc
                        .parents
                        .get(&id)
                        .map(|p| p.map(|p| doc.node_mappings.get(&p).copied()))
                        .flatten()
                        .flatten();

                    let key = tree.push(parent, node.element);

                    doc.node_mappings.insert(id, key);
                    events.insert(key, node.events);
                }
                Event::RemoveNode(id) => {
                    tracing::trace!("despawn node {:?}", id);

                    let mut delete_queue = vec![id];

                    let mut index = 0;
                    while index < delete_queue.len() {
                        let key = delete_queue[index];

                        doc.nodes.remove(key);
                        doc.parents.remove(&key);

                        if let Some(children) = doc.children.remove(&key) {
                            delete_queue.extend(children);
                        }

                        index += 1;
                    }

                    let mut delete_effects = vec![];

                    for node_id in delete_queue {
                        // Note that it is possible for the same node be queued for deletion
                        // twice if the parent and the children have both been requested
                        // for deletion manually in the same tick.
                        let Some(key) = doc.node_mappings.remove(&node_id) else {
                            continue 'out;
                        };

                        tree.remove(key);
                        events.remove(key);

                        // Remove effects registered on the node.
                        rt.effects.retain(|effect_id, effect| match effect.node {
                            Some(node) => {
                                if node == node_id {
                                    doc.effects.remove(&effect_id);
                                    delete_effects.push(effect_id);
                                    false
                                } else {
                                    true
                                }
                            }
                            None => true,
                        });
                    }

                    let mut delete_signals = vec![];

                    for id in delete_effects {
                        for (signal_id, effects) in rt.signal_effects.iter_mut() {
                            effects.retain(|effect_id| *effect_id != id);

                            if effects.len() == 0 {
                                delete_signals.push(*signal_id);
                            }
                        }
                    }

                    for id in delete_signals {
                        rt.signal_effects.remove(&id);
                    }
                }
                Event::UpdateNode(id, node) => {
                    tracing::trace!("replace node {:?}", id);

                    if let Some(key) = doc.node_mappings.get(&id) {
                        tree.replace(*key, node.element);
                        *events.get_mut(*key).unwrap() = node.events;
                    } else {
                        tracing::trace!("node {:?} does not exist", id);
                    }
                }
                Event::UpdateStyle(id, style) => {
                    tracing::trace!("update style {:?}", id);

                    if let Some(key) = doc.node_mappings.get(&id) {
                        tree.get_mut(*key).unwrap().style = style;
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
    PushNode(NodeId, Node),
    UpdateNode(NodeId, Node),
    RemoveNode(NodeId),
    UpdateStyle(NodeId, Style),
}

#[cfg(test)]
mod tests {
    use crate::events::{ElementEventHandlers, Events};
    use crate::reactive::Runtime;
    use crate::render::layout::LayoutTree;
    use crate::render::{Element, ElementBody};
    use crate::style::Style;

    use super::{Document, Node};

    pub(super) fn create_node() -> Node {
        Node {
            element: Element {
                body: ElementBody::Container(),
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

        assert!(doc.is_empty());
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

        assert!(doc.is_empty());
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
