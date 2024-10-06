use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::fmt::{self, Debug, Formatter};
use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::Arc;

use game_common::collections::arena::{Arena, Key};
use game_input::keyboard::KeyboardInput;
use game_input::mouse::{MouseButtonInput, MouseWheel};
use game_render::camera::RenderTarget;
use game_tracing::trace_span;
use game_window::events::{CursorMoved, WindowEvent};
use glam::UVec2;
use parking_lot::Mutex;

use crate::layout::{self, LayoutTree};
use crate::primitive::Primitive;
use crate::render::Rect;
use crate::widgets::Callback;
use crate::WindowProperties;

pub trait Widget: Sized + 'static {
    type Message;

    /// Updates the state of the widget with the new `msg` and returns whether the widget should be
    /// redrawn after the `update` call.
    ///
    /// The default implementation ignores the given `msg` and always returns `false`.
    // `#[allow(unused_variables)]` is preferred here instead of prefixing
    // them with underscores, so that all custom `update` implementations
    // still get the `unused_variables` warning if some variables are unused.
    #[allow(unused_variables)]
    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        false
    }

    /// Returns the [`View`] that should be rendered for this widget.
    fn view(&self, ctx: &Context<Self>) -> View;
}

trait RawWidget: 'static {
    fn update(&mut self, ctx: RawContext, msg: Box<dyn Any + Send + Sync + 'static>) -> bool;

    fn view(&self, ctx: RawContext) -> View;
}

impl<T> RawWidget for T
where
    T: Widget,
{
    fn update(&mut self, ctx: RawContext, msg: Box<dyn Any + Send + Sync + 'static>) -> bool {
        let ctx = Context {
            raw: ctx,
            _m: PhantomData,
        };
        T::update(self, &ctx, *msg.downcast().unwrap())
    }

    fn view(&self, ctx: RawContext) -> View {
        let ctx = Context {
            raw: ctx,
            _m: PhantomData,
        };
        T::view(self, &ctx)
    }
}

#[derive(Clone, Debug)]
pub struct Runtime {
    pub(crate) inner: Arc<Mutex<RuntimeInner>>,
}

impl Runtime {
    pub(crate) fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(RuntimeInner {
                windows: HashMap::new(),
                documents: Arena::new(),
            })),
        }
    }

    pub(crate) fn create_window(&self, target: RenderTarget, props: WindowProperties) {
        let _span = trace_span!("Runtime::create_window").entered();

        let mut rt = self.inner.lock();
        rt.windows.insert(
            target,
            Window {
                documents: Vec::new(),
                size: props.size,
                scale_factor: props.scale_factor,
                cursor_position: None,
            },
        );
    }

    pub(crate) fn destroy_window(&self, target: RenderTarget) {
        let _span = trace_span!("Runtime::destroy_window").entered();

        let mut rt = self.inner.lock();
        if let Some(window) = rt.windows.remove(&target) {
            for document in window.documents {
                rt.documents.remove(document.0);
            }
        }
    }

    pub(crate) fn resize_window(&self, target: RenderTarget, size: UVec2) {
        let rt = &mut *self.inner.lock();
        if let Some(window) = rt.windows.get_mut(&target) {
            window.size = size;
            for document in &window.documents {
                let document = rt.documents.get_mut(document.0).unwrap();
                document.tree.resize(size);
            }
        }
    }

    pub(crate) fn update_scale_factor(&self, target: RenderTarget, scale_factor: f64) {
        let rt = &mut *self.inner.lock();
        if let Some(window) = rt.windows.get_mut(&target) {
            window.scale_factor = scale_factor;
            for document in &window.documents {
                let document = rt.documents.get_mut(document.0).unwrap();
                document.tree.set_scale_factor(scale_factor);
            }
        }
    }

    pub fn mount<T>(&self, window: RenderTarget, root: T) -> DocumentId
    where
        T: Widget,
    {
        let _span = trace_span!("Runtime::mount").entered();

        let mut rt = self.inner.lock();
        assert!(rt.windows.contains_key(&window));

        let mut document = Document::new(window);
        let root_key = document.nodes.insert(Node {
            node: Rc::new(RefCell::new(root)),
            layout_key: None,
            event_handlers: HashMap::new(),
        });

        let key = rt.documents.insert(document);

        let window = rt.windows.get_mut(&window).unwrap();
        window.documents.push(DocumentId(key));

        drop(rt);
        self.render_document(DocumentId(key), root_key);

        DocumentId(key)
    }

    pub fn unmount(&self, document_id: DocumentId) {
        let _span = trace_span!("Runtime::unmount").entered();

        let mut rt = self.inner.lock();
        let Some(document) = rt.documents.remove(document_id.0) else {
            return;
        };

        let window = rt.windows.get_mut(&document.window).unwrap();
        window.documents.retain(|id| *id != document_id);
    }

    pub(crate) fn send_event(&self, window: RenderTarget, event: WindowEvent) {
        match event {
            WindowEvent::CursorEntered(_) => {}
            WindowEvent::CursorLeft(_) => {
                let mut rt = self.inner.lock();
                if let Some(window) = rt.windows.get_mut(&window) {
                    window.cursor_position = None;
                }
            }
            WindowEvent::CursorMoved(event) => {
                {
                    let mut rt = self.inner.lock();
                    if let Some(window) = rt.windows.get_mut(&window) {
                        window.cursor_position = Some(event.position.as_uvec2());
                    }
                }

                self.send_event_inner(window, event);
            }
            WindowEvent::MouseButtonInput(event) => {
                self.send_event_inner(window, event);
            }
            WindowEvent::MouseWheel(event) => {
                self.send_event_inner(window, event);
            }
            WindowEvent::KeyboardInput(event) => {
                self.send_event_inner(window, event);
            }
            _ => (),
        }
    }

    pub fn send_event_inner<E>(&self, window: RenderTarget, event: E)
    where
        E: Event + Clone,
    {
        let rt = self.inner.lock();
        let Some(window) = rt.windows.get(&window) else {
            return;
        };

        for document in &window.documents {
            let document = rt.documents.get(document.0).unwrap();

            for node in document.nodes.values() {
                let Some(handler) = node.event_handlers.get(&TypeId::of::<E>()) else {
                    continue;
                };

                handler(Box::new(event.clone()));
            }
        }
    }

    pub(crate) fn update(&self) {
        let _span = trace_span!("Runtime::update").entered();

        let mut rt = self.inner.lock();

        let mut msg_queue = Vec::new();

        for (document_id, document) in &mut rt.documents {
            for event in document.events.lock().drain(..) {
                match event {
                    NodeEvent::SendMessage(key, msg) => {
                        let Some(node) = document.nodes.get_mut(key) else {
                            continue;
                        };

                        msg_queue.push((document_id, key, node.node.clone(), msg));
                    }
                }
            }
        }

        drop(rt);
        let mut update_queue = Vec::new();

        for (document, key, node, msg) in msg_queue.drain(..) {
            let events = {
                self.inner
                    .lock()
                    .documents
                    .get_mut(document)
                    .unwrap()
                    .events
                    .clone()
            };

            let ctx = RawContext {
                document: DocumentId(document),
                node: key,
                events,
                runtime: self.clone(),
            };

            if node.borrow_mut().update(ctx, msg) {
                update_queue.push((document, key));
            }
        }

        // A widget may receive a message multiple times in an update cycle.
        // In that case it should still only be re-rendered once.
        update_queue.dedup();

        let mut rt = self.inner.lock();
        for (document_id, node) in &update_queue {
            let document = rt.documents.get_mut(*document_id).unwrap();
            document.remove_children(*node);
        }

        drop(rt);
        for (document_id, node) in update_queue {
            self.render_document(DocumentId(document_id), node);
        }
    }

    fn render_document(&self, doc_key: DocumentId, node: Key) {
        let _span = trace_span!("Runtime::render_document").entered();

        let mut queue = VecDeque::new();
        queue.push_back(node);

        while let Some(node_key) = queue.pop_front() {
            let mut rt = self.inner.lock();
            let document = rt.documents.get_mut(doc_key.0).unwrap();
            let node = document.nodes.get(node_key).unwrap().node.clone();
            let events = document.events.clone();
            drop(rt);

            let ctx = RawContext {
                document: doc_key,
                node: node_key,
                events,
                runtime: self.clone(),
            };

            let view = node.borrow().view(ctx);

            let mut rt = self.inner.lock();
            let document = rt.documents.get_mut(doc_key.0).unwrap();

            if let Some(primitive) = view.primitive {
                let parent = document.find_layout_parent(node_key);
                let key = document.tree.push(parent, primitive);

                let node = document.nodes.get_mut(node_key).unwrap();
                node.layout_key = Some(key);
            }

            if let Some(node_ref) = view.node_ref {
                document.node_refs.insert(node_ref.0.id, node_key);
            }

            for node in view.children.0 {
                let child_key = document.nodes.insert(Node {
                    node,
                    layout_key: None,
                    event_handlers: HashMap::new(),
                });

                document.parents.insert(child_key, node_key);
                document
                    .children
                    .entry(node_key)
                    .or_default()
                    .push(child_key);

                queue.push_back(child_key);
            }
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct DocumentId(pub(crate) Key);

#[derive(Debug)]
pub(crate) struct RuntimeInner {
    pub(crate) windows: HashMap<RenderTarget, Window>,
    pub(crate) documents: Arena<Document>,
}

#[derive(Clone, Debug)]
pub(crate) struct Window {
    pub(crate) documents: Vec<DocumentId>,
    size: UVec2,
    scale_factor: f64,
    cursor_position: Option<UVec2>,
}

#[derive(Debug)]
pub(crate) struct Document {
    window: RenderTarget,
    pub(crate) tree: LayoutTree,
    nodes: Arena<Node>,
    events: Arc<Mutex<VecDeque<NodeEvent>>>,
    // Parent ==> Children
    children: HashMap<Key, Vec<Key>>,
    // Child ==> Parent
    parents: HashMap<Key, Key>,
    next_node_ref: u64,
    // NodeRef ==> Node
    node_refs: HashMap<NodeRefId, Key>,
    custom_data: HashMap<TypeId, Rc<dyn Any + 'static>>,
}

impl Document {
    fn new(window: RenderTarget) -> Self {
        Self {
            window,
            tree: LayoutTree::new(),
            nodes: Arena::new(),
            children: HashMap::new(),
            events: Default::default(),
            parents: HashMap::new(),
            next_node_ref: 0,
            node_refs: HashMap::new(),
            custom_data: HashMap::new(),
        }
    }

    fn remove_children(&mut self, key: Key) {
        if let Some(node) = self.nodes.get(key) {
            if let Some(key) = node.layout_key {
                self.tree.remove(key);
            }
        }

        let mut despawn_queue = Vec::new();

        if let Some(c) = self.children.remove(&key) {
            despawn_queue.extend(c);
        }

        while let Some(key) = despawn_queue.pop() {
            self.parents.remove(&key);

            if let Some(node) = self.nodes.get(key) {
                if let Some(key) = node.layout_key {
                    self.tree.remove(key);
                }
            }

            if let Some(c) = self.children.remove(&key) {
                despawn_queue.extend(c);
            }
        }
    }

    fn find_layout_parent(&self, key: Key) -> Option<layout::Key> {
        let mut key = self.parents.get(&key)?;

        loop {
            let node = self.nodes.get(*key)?;
            if let Some(key) = node.layout_key {
                return Some(key);
            }

            key = self.parents.get(&key)?;
        }
    }
}

struct Node {
    node: Rc<RefCell<dyn RawWidget>>,
    layout_key: Option<layout::Key>,
    event_handlers: HashMap<TypeId, Rc<dyn Fn(Box<dyn Any + Send + Sync + 'static>)>>,
}

impl Debug for Node {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Node")
            .field("layout_key", &self.layout_key)
            .finish_non_exhaustive()
    }
}

#[derive(Clone)]
pub struct View {
    pub(crate) primitive: Option<Primitive>,
    pub(crate) children: Children,
    pub(crate) node_ref: Option<NodeRef>,
}

impl<T> From<T> for View
where
    T: Widget,
{
    fn from(value: T) -> Self {
        Self {
            primitive: None,
            children: Children::from(value),
            node_ref: None,
        }
    }
}

impl<const N: usize, T> From<[T; N]> for View
where
    T: Widget,
{
    fn from(value: [T; N]) -> Self {
        Self {
            primitive: None,
            children: Children::from(value),
            node_ref: None,
        }
    }
}

impl<T> From<Vec<T>> for View
where
    T: Widget,
{
    fn from(value: Vec<T>) -> Self {
        Self {
            primitive: None,
            children: Children::from(value),
            node_ref: None,
        }
    }
}

struct RawContext {
    document: DocumentId,
    node: Key,
    events: Arc<Mutex<VecDeque<NodeEvent>>>,
    runtime: Runtime,
}

pub struct Context<T>
where
    T: ?Sized,
{
    raw: RawContext,
    _m: PhantomData<T>,
}

impl<T> Context<T>
where
    T: ?Sized + Widget,
{
    pub fn callback<I, F>(&self, f: F) -> Callback<I>
    where
        F: Fn(I) -> T::Message + Send + Sync + 'static,
        T::Message: Send + Sync + 'static,
    {
        let node = self.raw.node;
        let events = self.raw.events.clone();

        Callback::from(move |value| {
            let msg = f(value);

            events
                .lock()
                .push_back(NodeEvent::SendMessage(node, Box::new(msg)));
        })
    }

    pub fn on_event<E, F>(&self, f: F) -> EventHandlerHandle
    where
        F: Fn(E) -> T::Message + 'static,
        E: Event,
        T::Message: Send + Sync + 'static,
    {
        let mut rt = self.raw.runtime.inner.lock();
        let document = rt.documents.get_mut(self.raw.document.0).unwrap();
        let node = document.nodes.get_mut(self.raw.node).unwrap();
        node.event_handlers.insert(TypeId::of::<E>(), {
            let node = self.raw.node;
            let events = self.raw.events.clone();

            Rc::new(move |value| {
                let msg = f(*value.downcast().unwrap());

                events
                    .lock()
                    .push_back(NodeEvent::SendMessage(node, Box::new(msg)));
            })
        });

        EventHandlerHandle {
            document: self.raw.document,
            node: self.raw.node,
            event: TypeId::of::<E>(),
            runtime: self.raw.runtime.clone(),
        }
    }

    pub fn create_node_ref(&self) -> NodeRef {
        let mut rt = self.raw.runtime.inner.lock();
        let document = rt.documents.get_mut(self.raw.document.0).unwrap();
        let id = NodeRefId(document.next_node_ref);
        document.next_node_ref += 1;

        NodeRef(Rc::new(NodeRefInner {
            document: self.raw.document,
            id,
            runtime: self.raw.runtime.clone(),
        }))
    }

    pub fn layout(&self, node: &NodeRef) -> Option<Rect> {
        let rt = self.raw.runtime.inner.lock();
        let document = rt.documents.get(node.0.document.0)?;
        let node_id = document.node_refs.get(&node.0.id)?;
        let node = document.nodes.get(*node_id)?;

        // If the node itself has a layout we can simply
        // use that layout.
        if let Some(layout_key) = node.layout_key {
            return document.tree.layout(layout_key).map(|layout| Rect {
                min: layout.position,
                max: UVec2 {
                    x: layout.position.x + layout.width,
                    y: layout.position.y + layout.height,
                },
            });
        }

        // Otherwise we must compute the joined layout of all children.

        let mut layout = Rect {
            min: UVec2::MAX,
            max: UVec2::ZERO,
        };
        let mut children_queued: VecDeque<Key> = VecDeque::new();
        let Some(children) = document.children.get(node_id) else {
            return None;
        };
        children_queued.extend(children);

        while let Some(key) = children_queued.pop_front() {
            let node = document.nodes.get(key)?;

            if let Some(layout_key) = node.layout_key {
                // If a node has a layout we do not need to check its children
                // since they are already included in the layout.
                let child_layout = document.tree.layout(layout_key).unwrap();
                layout.min = UVec2::min(layout.min, child_layout.position);
                layout.max = UVec2::max(
                    layout.max,
                    UVec2::new(
                        child_layout.position.x + child_layout.width,
                        child_layout.position.y + child_layout.height,
                    ),
                )
            } else {
                if let Some(children) = document.children.get(node_id) {
                    children_queued.extend(children);
                }
            }
        }

        if layout.min != UVec2::MIN || layout.max != UVec2::MAX {
            Some(layout)
        } else {
            None
        }
    }

    pub fn cursor(&self) -> CursorRef<'_> {
        CursorRef { ctx: &self.raw }
    }

    pub fn custom_data(&self) -> CustomData<'_> {
        CustomData { ctx: &self.raw }
    }

    pub fn clipboard(&self) -> ClipboardRef<'_> {
        ClipboardRef { ctx: &self.raw }
    }

    // pub fn get(&self) -> NodeRef<'_> {
    //     NodeRef {
    //         runtime: &self.raw.runtime,
    //     }
    // }
}

pub struct CursorRef<'a> {
    ctx: &'a RawContext,
}

impl<'a> CursorRef<'a> {
    pub fn position(&self) -> Option<UVec2> {
        let rt = self.ctx.runtime.inner.lock();
        let document = rt.documents.get(self.ctx.document.0)?;
        let window = rt.windows.get(&document.window)?;
        window.cursor_position
    }
}

pub struct CustomData<'a> {
    ctx: &'a RawContext,
}

impl<'a> CustomData<'a> {
    pub fn insert<T>(&self, data: T)
    where
        T: 'static,
    {
        let mut rt = self.ctx.runtime.inner.lock();
        let document = rt.documents.get_mut(self.ctx.document.0).unwrap();
        document
            .custom_data
            .insert(TypeId::of::<T>(), Rc::new(data));
    }

    pub fn get<T>(&self) -> Option<Rc<T>>
    where
        T: 'static,
    {
        let mut rt = self.ctx.runtime.inner.lock();
        let document = rt.documents.get_mut(self.ctx.document.0).unwrap();
        document
            .custom_data
            .get(&TypeId::of::<T>())
            .map(|v| v.clone().downcast().unwrap())
    }

    pub fn remove<T>(&self)
    where
        T: 'static,
    {
        let mut rt = self.ctx.runtime.inner.lock();
        let document = rt.documents.get_mut(self.ctx.document.0).unwrap();
        document.custom_data.remove(&TypeId::of::<T>());
    }
}

pub struct ClipboardRef<'a> {
    ctx: &'a RawContext,
}

impl<'a> ClipboardRef<'a> {
    pub fn get(&self) -> Option<String> {
        todo!()
    }

    pub fn set(&self, value: &str) {
        todo!()
    }
}

// struct NodeRef<'a> {
//     runtime: &'a Runtime,
// }

// impl<'a> NodeRef<'a> {
//     pub fn layout(&self) -> UVec2 {}
// }

pub trait Event: Sized + Send + Sync + 'static {}

impl Event for CursorMoved {}
impl Event for KeyboardInput {}
impl Event for MouseButtonInput {}
impl Event for MouseWheel {}

#[derive(Debug)]
pub struct EventHandlerHandle {
    runtime: Runtime,
    document: DocumentId,
    node: Key,
    event: TypeId,
}

impl Drop for EventHandlerHandle {
    fn drop(&mut self) {
        let mut rt = self.runtime.inner.lock();
        let document = rt.documents.get_mut(self.document.0).unwrap();
        let node = document.nodes.get_mut(self.node).unwrap();
        node.event_handlers.remove(&self.event);
    }
}

enum NodeEvent {
    SendMessage(Key, Box<dyn Any + Send + Sync + 'static>),
}

impl Debug for NodeEvent {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("NodeEvent").finish_non_exhaustive()
    }
}

#[derive(Clone, Debug)]
pub struct NodeRef(Rc<NodeRefInner>);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
struct NodeRefId(u64);

#[derive(Debug)]
struct NodeRefInner {
    document: DocumentId,
    id: NodeRefId,
    runtime: Runtime,
}

impl Drop for NodeRefInner {
    fn drop(&mut self) {
        let mut rt = self.runtime.inner.lock();
        if let Some(document) = rt.documents.get_mut(self.document.0) {
            document.node_refs.remove(&self.id);
        }
    }
}

#[derive(Clone, Default)]
pub struct Children(Vec<Rc<RefCell<dyn RawWidget>>>);

impl Children {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn push<T>(&mut self, widget: T)
    where
        T: Widget,
    {
        self.0.push(Rc::new(RefCell::new(widget)));
    }
}

impl<T> From<T> for Children
where
    T: Widget,
{
    fn from(value: T) -> Self {
        Self(vec![Rc::new(RefCell::new(value))])
    }
}

impl<T, const N: usize> From<[T; N]> for Children
where
    T: Widget,
{
    fn from(value: [T; N]) -> Self {
        let mut children = Vec::with_capacity(N);
        for val in value {
            children.push(Rc::new(RefCell::new(val)) as Rc<RefCell<dyn RawWidget>>);
        }

        Self(children)
    }
}

impl<T> From<Vec<T>> for Children
where
    T: Widget,
{
    fn from(value: Vec<T>) -> Self {
        let mut children = Vec::with_capacity(value.len());
        for val in value {
            children.push(Rc::new(RefCell::new(val)) as Rc<RefCell<dyn RawWidget>>);
        }

        Self(children)
    }
}
