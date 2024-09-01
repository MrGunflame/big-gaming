use std::alloc::Layout;
use std::any::TypeId;
use std::collections::HashMap;
use std::future::Future;
use std::marker::PhantomData;
use std::mem::{ManuallyDrop, MaybeUninit};
use std::ptr::NonNull;
use std::sync::Arc;

use game_common::collections::arena::{self, Arena};
use game_render::camera::RenderTarget;
use game_tasks::TaskPool;
use game_tracing::trace_span;
use game_window::cursor::Cursor;
use glam::{UVec2, Vec2};
use parking_lot::Mutex;

use crate::clipboard::Clipboard;
use crate::layout::{self, LayoutTree};
use crate::primitive::Primitive;
use crate::render::Rect;

#[derive(Clone, Debug)]
pub struct Runtime {
    pub(crate) inner: Arc<Mutex<RuntimeInner>>,
    pub(crate) cursor: Arc<Mutex<Option<Arc<Cursor>>>>,
    // FIXME: We probably want to share the same task pool
    // instead of having a separate one just for UI.
    pool: Arc<TaskPool>,
    clipboard: Arc<Mutex<Clipboard>>,
}

impl Runtime {
    pub(crate) fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(RuntimeInner {
                windows: HashMap::new(),
                documents: Arena::new(),
                nodes: Arena::new(),
                hierarchy: NodeHierarchy::default(),
                event_handlers: Arena::new(),
                event_handler_parents: HashMap::new(),
            })),
            cursor: Arc::new(Mutex::new(None)),
            pool: Arc::new(TaskPool::new(1)),
            clipboard: Arc::new(Mutex::new(Clipboard::new())),
        }
    }

    pub fn documents(&self, window: RenderTarget) -> Vec<DocumentId> {
        let _span = trace_span!("Runtime::documents").entered();

        let rt = self.inner.lock();
        rt.windows
            .get(&window)
            .map(|w| w.documents.clone())
            .unwrap_or(Vec::new())
    }

    pub fn append(
        &self,
        document: DocumentId,
        parent: Option<NodeId>,
        mut node: Node,
    ) -> Option<NodeId> {
        let _span = trace_span!("Runtime::append").entered();

        let document_id = document;

        let rt = &mut *self.inner.lock();
        let document = rt.documents.get_mut(document.0)?;

        let node_key = if let Some(parent) = parent {
            rt.nodes.get(parent.0)?;

            let parent_key = document.layout_node_map.get(&parent).unwrap();
            document
                .layout
                .push(Some(*parent_key), node.primitive.clone().into())
        } else {
            document.layout.push(None, node.primitive.clone().into())
        };

        node.document = Some(document_id);
        let id = NodeId(rt.nodes.insert(node));

        if let Some(parent) = parent {
            rt.hierarchy.children.entry(parent).or_default().push(id);
            rt.hierarchy.parents.insert(id, parent);
        }

        document.layout_node_map.insert(id, node_key);
        document.layout_node_map2.insert(node_key, id);

        if parent.is_none() {
            document.root_nodes.push(id);
        }

        Some(id)
    }

    pub fn create_document(&self, window: RenderTarget) -> Option<DocumentId> {
        let _span = trace_span!("Runtime::create_document").entered();

        let rt = &mut *self.inner.lock();
        let window = rt.windows.get_mut(&window)?;

        let doc = DocumentId(rt.documents.insert(Document {
            layout: LayoutTree::new(),
            layout_node_map: HashMap::new(),
            layout_node_map2: HashMap::new(),
            event_handlers: EventHandlers::default(),
            type_map: HashMap::new(),
            root_nodes: Vec::new(),
        }));

        window.documents.push(doc);
        Some(doc)
    }

    pub fn clear_children(&self, node: NodeId) {
        let _span = trace_span!("Runtime::clear_children").entered();

        let children = {
            let rt = &mut *self.inner.lock();

            let Some(children) = rt.hierarchy.children.get(&node) else {
                return;
            };

            children.to_vec()
        };

        for child in children {
            self.remove(child);
        }
    }

    pub fn remove(&self, node: NodeId) {
        let _span = trace_span!("Runtime::remove").entered();

        let mut node_destroyed_handlers = Vec::new();

        // The document of the destroyed nodes.
        // Note that this is the same for all nodes that
        // are destoryed.
        let mut document = None;

        {
            let rt = &mut *self.inner.lock();

            let mut event_handlers_to_destroy = Vec::new();
            let mut children = vec![node];
            while let Some(node_id) = children.pop() {
                if let Some(handlers) = rt.event_handler_parents.remove(&node_id) {
                    // If the node has a `NodeDestroyed` attached, we must call
                    // it after destroying it.
                    for handler in &handlers {
                        let handler = rt.event_handlers.get(handler.0).unwrap();
                        if handler.event == TypeId::of::<NodeDestroyed>() {
                            node_destroyed_handlers.push((node_id, handler.handler.clone()));
                        }
                    }

                    event_handlers_to_destroy.extend(handlers);
                }

                let Some(node) = rt.nodes.remove(node_id.0) else {
                    continue;
                };

                document = node.document;

                if let Some(parent) = rt.hierarchy.parents.remove(&node_id) {
                    if let Some(children) = rt.hierarchy.children.get_mut(&parent) {
                        children.retain(|child| *child != node_id);
                    }
                }

                if let Some(c) = rt.hierarchy.children.remove(&node_id) {
                    children.extend(c);
                }

                let doc = rt.documents.get_mut(node.document.unwrap().0).unwrap();
                let key = doc.layout_node_map.remove(&node_id).unwrap();
                doc.layout_node_map2.remove(&key).unwrap();
                doc.layout.remove(key);

                // Remove the node from the document root nodes if it is one.
                doc.root_nodes.retain(|n| *n != node_id);
            }

            for handler in event_handlers_to_destroy {
                rt.unregister(handler);
            }
        }

        let Some(document) = document else {
            return;
        };

        for (node, handler) in node_destroyed_handlers {
            // SAFETY: We already checked that the handlers are
            // for E: NodeDestroyed.
            unsafe {
                handler.lock().call(Context {
                    event: NodeDestroyed,
                    node: Some(node),
                    document,
                    runtime: self.clone(),
                });
            }
        }
    }

    pub(crate) fn create_window(&self, id: RenderTarget, size: UVec2) {
        let _span = trace_span!("Runtime::create_window").entered();

        let mut rt = self.inner.lock();
        rt.windows.insert(
            id,
            Window {
                documents: Vec::new(),
                size,
            },
        );
    }

    pub(crate) fn resize_window(&self, id: RenderTarget, size: UVec2) {
        let _span = trace_span!("Runtime::resize_window").entered();

        let rt = &mut *self.inner.lock();
        let window = rt.windows.get_mut(&id).unwrap();
        window.size = size;

        for doc in &window.documents {
            rt.documents.get_mut(doc.0).unwrap().layout.resize(size);
        }
    }

    pub(crate) fn destroy_window(&self, id: RenderTarget) {
        let _span = trace_span!("Runtime::destroy_window").entered();

        let mut rt = self.inner.lock();
        if let Some(window) = rt.windows.remove(&id) {
            drop(rt);
            for id in window.documents {
                self.destroy_document(id);
            }
        }
    }

    pub fn root_context(&self, document: DocumentId) -> Context<()> {
        Context {
            event: (),
            node: None,
            document,
            runtime: self.clone(),
        }
    }

    pub fn spawn_task<F>(&self, future: F) -> TaskHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let handle = self.pool.spawn(future);
        TaskHandle(ManuallyDrop::new(handle))
    }

    pub(crate) fn clipboard_get(&self) -> Option<String> {
        self.clipboard.lock().get()
    }

    pub(crate) fn clipboard_set(&self, value: &str) {
        self.clipboard.lock().set(value);
    }

    fn register_on_document<E, F>(&self, document: DocumentId, parent: Option<NodeId>, handler: F)
    where
        F: FnMut(Context<E>) + Send + Sync + 'static,
        E: Event,
    {
        let _span = trace_span!("Runtime::register_on_document").entered();

        if TypeId::of::<E>() == TypeId::of::<NodeDestroyed>() {
            assert!(
                parent.is_some(),
                "NodeDestroyed event handlers must be attached to a node"
            );
        }

        let mut rt = self.inner.lock();

        let entry = EventHandlerEntry {
            handler: Arc::new(Mutex::new(EventHandlerPtr::new(handler))),
            document,
            event: TypeId::of::<E>(),
        };

        let id = EventHandlerId(rt.event_handlers.insert(entry));

        let doc = rt.documents.get_mut(document.0).unwrap();
        doc.event_handlers.insert::<E>(id);

        if let Some(parent) = parent {
            rt.event_handler_parents.entry(parent).or_default().push(id);
        }
    }

    fn destroy_document(&self, id: DocumentId) {
        let _span = trace_span!("Runtime::destroy_document").entered();

        let rt = self.inner.lock();

        let Some(document) = rt.documents.get(id.0) else {
            return;
        };

        let nodes = document.root_nodes.clone();
        drop(rt);

        for node in nodes {
            self.remove(node);
        }

        let mut rt = self.inner.lock();
        rt.documents.remove(id.0);
    }
}

#[derive(Debug)]
pub(crate) struct RuntimeInner {
    pub(crate) windows: HashMap<RenderTarget, Window>,
    pub(crate) documents: Arena<Document>,
    pub(crate) nodes: Arena<Node>,
    hierarchy: NodeHierarchy,
    event_handlers: Arena<EventHandlerEntry>,
    event_handler_parents: HashMap<NodeId, Vec<EventHandlerId>>,
}

impl RuntimeInner {
    pub(crate) fn get_event_handler<E>(&self, id: EventHandlerId) -> EventHandler<E>
    where
        E: Event,
    {
        let entry = self.event_handlers.get(id.0).unwrap();
        assert_eq!(TypeId::of::<E>(), entry.event);

        EventHandler {
            ptr: entry.handler.clone(),
            _marker: PhantomData,
        }
    }

    fn unregister(&mut self, id: EventHandlerId) {
        let entry = self.event_handlers.remove(id.0).unwrap();

        self.documents
            .get_mut(entry.document.0)
            .unwrap()
            .event_handlers
            .remove(id);
    }
}

#[derive(Clone, Debug)]
struct EventHandlerEntry {
    handler: Arc<Mutex<EventHandlerPtr>>,
    document: DocumentId,
    event: TypeId,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct EventHandlerId(arena::Key);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct DocumentId(pub(crate) arena::Key);

#[derive(Debug)]
pub struct Window {
    pub documents: Vec<DocumentId>,
    size: UVec2,
}

#[derive(Debug)]
pub struct Document {
    pub(crate) event_handlers: EventHandlers,
    pub(crate) layout: LayoutTree,
    pub(crate) layout_node_map: HashMap<NodeId, layout::Key>,
    pub(crate) layout_node_map2: HashMap<layout::Key, NodeId>,
    pub(crate) type_map: HashMap<TypeId, Arc<dyn std::any::Any + Send + Sync + 'static>>,
    pub(crate) root_nodes: Vec<NodeId>,
}

#[derive(Debug)]
pub struct Node {
    primitive: Primitive,
    event_handlers: EventHandlers,
    document: Option<DocumentId>,
}

impl Node {
    pub fn new(primitive: Primitive) -> Self {
        Self {
            primitive,
            event_handlers: EventHandlers::default(),
            document: None,
        }
    }

    // pub fn register<E, F>(&mut self, handler: F)
    // where
    //     F: FnMut(Context<E>) + Send + Sync + 'static,
    //     E: Event,
    // {
    //     self.event_handlers.insert(handler);
    // }

    // pub(crate) fn get<E>(&self) -> Option<Vec<EventHandler<E>>>
    // where
    //     E: Event,
    // {
    //     self.event_handlers.get()
    // }
}

struct Header {
    call: unsafe fn(NonNull<()>, *const ()),
    drop: unsafe fn(NonNull<()>),
    layout: Layout,
}

#[repr(C)]
struct RawEventHandler<E> {
    header: Header,
    handler: ManuallyDrop<Box<dyn FnMut(Context<E>) + Send + Sync + 'static>>,
}

impl<E> RawEventHandler<E> {
    const LAYOUT: Layout = Layout::new::<Self>();

    unsafe fn call(ptr: NonNull<()>, event: *const ()) {
        unsafe {
            let this = ptr.cast::<Self>().as_mut();
            let event = event.cast::<Context<E>>().read();

            (this.handler)(event);
        }
    }

    unsafe fn drop(ptr: NonNull<()>) {
        unsafe {
            let this = ptr.cast::<Self>().as_mut();
            ManuallyDrop::drop(&mut this.handler);
        }
    }
}

#[derive(Debug)]
struct EventHandlerPtr {
    ptr: NonNull<()>,
}

impl EventHandlerPtr {
    fn new<E, F>(f: F) -> Self
    where
        F: FnMut(Context<E>) + Send + Sync + 'static,
    {
        let layout = RawEventHandler::<E>::LAYOUT;
        let ptr = unsafe {
            let ptr = std::alloc::alloc(layout);
            if ptr.is_null() {
                std::alloc::handle_alloc_error(layout);
            }

            ptr.cast::<RawEventHandler<E>>()
                .write(RawEventHandler::<E> {
                    header: Header {
                        call: RawEventHandler::<E>::call,
                        drop: RawEventHandler::<E>::drop,
                        layout: RawEventHandler::<E>::LAYOUT,
                    },
                    handler: ManuallyDrop::new(Box::new(f)),
                });

            NonNull::new_unchecked(ptr).cast::<()>()
        };

        Self { ptr }
    }

    unsafe fn call<E>(&mut self, event: Context<E>)
    where
        E: Event,
    {
        let event = MaybeUninit::new(event);

        unsafe {
            let header = self.ptr.cast::<Header>().as_ref();
            (header.call)(self.ptr, event.as_ptr().cast::<()>());
        }
    }
}

impl Drop for EventHandlerPtr {
    fn drop(&mut self) {
        unsafe {
            let header = self.ptr.cast::<Header>().as_ref();
            (header.drop)(self.ptr);

            let layout = header.layout;
            std::alloc::dealloc(self.ptr.as_ptr().cast::<u8>(), layout);
        }
    }
}

unsafe impl Send for EventHandlerPtr {}
unsafe impl Sync for EventHandlerPtr {}

pub(crate) struct EventHandler<E> {
    ptr: Arc<Mutex<EventHandlerPtr>>,
    _marker: PhantomData<fn(E)>,
}

impl<E> EventHandler<E>
where
    E: Event,
{
    pub fn call(&self, event: Context<E>) {
        let _span = trace_span!("EventHandler::call").entered();

        unsafe {
            self.ptr.lock().call(event);
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct EventHandlers {
    map: HashMap<TypeId, Vec<EventHandlerId>>,
}

impl EventHandlers {
    pub(crate) fn get<E>(&self) -> Option<&Vec<EventHandlerId>>
    where
        E: Event,
    {
        self.map.get(&TypeId::of::<E>())
    }

    fn insert<E>(&mut self, id: EventHandlerId)
    where
        E: Event,
    {
        self.map.entry(TypeId::of::<E>()).or_default().push(id);
    }

    fn remove(&mut self, id: EventHandlerId) {
        self.map.retain(|_, entries| {
            entries.retain(|e| *e != id);
            !entries.is_empty()
        });
    }
}

pub trait Event: Sized + Send + Sync + 'static {}

/// Event that is fired once the node has been destroyed.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct NodeDestroyed;

impl Event for NodeDestroyed {}

#[derive(Clone, Debug)]
pub struct Context<E> {
    pub event: E,
    pub(crate) node: Option<NodeId>,
    pub(crate) document: DocumentId,
    pub(crate) runtime: Runtime,
}

impl<E> Context<E> {
    pub fn append(&self, node: Node) -> Context<()> {
        let node = self.runtime.append(self.document, self.node, node).unwrap();
        Context {
            event: (),
            node: Some(node),
            document: self.document,
            runtime: self.runtime.clone(),
        }
    }

    pub fn runtime(&self) -> &Runtime {
        &self.runtime
    }

    pub fn remove(&self, node: NodeId) {
        self.runtime.remove(node);
    }

    /// Removes the node references by this `Context`.
    ///
    /// Note that some operations that implicitly use the node of this `Context` may fail after
    /// calling `remove_self`. Reuse of the `Context` should be avoided after calling `remove_self`.
    /// This function takes `self` by value for this reason.
    pub fn remove_self(self) {
        if let Some(node) = self.node {
            self.remove(node);
        }
    }

    pub fn clear_children(&self) {
        if let Some(node) = self.node {
            self.runtime.clear_children(node);
        }
    }

    pub fn document(&self) -> DocumentRef<'_> {
        DocumentRef {
            rt: &self.runtime,
            id: self.document,
        }
    }

    pub fn cursor(&self) -> Vec2 {
        match self.runtime.cursor.lock().as_ref() {
            Some(cursor) => cursor.position(),
            None => Vec2::ZERO,
        }
    }

    pub fn node(&self) -> Option<NodeId> {
        self.node
    }

    pub fn layout(&self, node: NodeId) -> Option<Rect> {
        let _span = trace_span!("Context::layout").entered();

        let mut rt = self.runtime.inner.lock();
        let doc = rt.documents.get_mut(self.document.0)?;
        doc.layout.compute_layout();

        let key = doc.layout_node_map.get(&node)?;
        let layout = doc.layout.layout(*key).unwrap();
        Some(Rect {
            min: layout.position,
            max: UVec2 {
                x: layout.position.x + layout.width,
                y: layout.position.y + layout.height,
            },
        })
    }

    pub fn with_event<U>(self, event: U) -> Context<U> {
        Context {
            event,
            node: self.node,
            document: self.document,
            runtime: self.runtime,
        }
    }
}

pub struct DocumentRef<'a> {
    rt: &'a Runtime,
    id: DocumentId,
}

impl<'a> DocumentRef<'a> {
    pub fn register<E, F>(&self, handler: F)
    where
        F: FnMut(Context<E>) + Send + Sync + 'static,
        E: Event,
    {
        self.rt.register_on_document(self.id, None, handler);
    }

    pub fn register_with_parent<E, F>(&self, parent: NodeId, handler: F)
    where
        F: FnMut(Context<E>) + Send + Sync + 'static,
        E: Event,
    {
        self.rt.register_on_document(self.id, Some(parent), handler);
    }

    pub fn get<T>(&self) -> Option<Arc<T>>
    where
        T: Send + Sync + 'static,
    {
        let _span = trace_span!("DocumentRef::get").entered();

        let rt = self.rt.inner.lock();
        let doc = rt.documents.get(self.id.0)?;
        doc.type_map
            .get(&TypeId::of::<T>())
            .map(|v| v.clone().downcast().unwrap())
    }

    pub fn insert<T>(&self, value: T)
    where
        T: Send + Sync + 'static,
    {
        let _span = trace_span!("DocumentRef::insert").entered();

        let mut rt = self.rt.inner.lock();
        let doc = rt.documents.get_mut(self.id.0).unwrap();
        doc.type_map.insert(TypeId::of::<T>(), Arc::new(value));
    }

    pub fn remove<T>(&self) -> Option<Arc<T>>
    where
        T: Send + Sync + 'static,
    {
        let _span = trace_span!("DocumentRef::remove").entered();

        let mut rt = self.rt.inner.lock();
        let doc = rt.documents.get_mut(self.id.0)?;
        doc.type_map
            .remove(&TypeId::of::<T>())
            .map(|v| v.clone().downcast().unwrap())
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct NodeId(pub(crate) arena::Key);

#[derive(Clone, Debug, Default)]
struct NodeHierarchy {
    children: HashMap<NodeId, Vec<NodeId>>,
    parents: HashMap<NodeId, NodeId>,
}

impl NodeHierarchy {
    // pub fn is_empty(&self) -> bool {
    //     self.len() == 0
    // }

    // pub fn push(&mut self, parent: Option<NodeId>) -> NodeId {
    //     let key = self.nodes.insert(None);

    //     if let Some(parent) = parent {
    //         debug_assert!(self.nodes.contains_key(parent));

    //         self.parents.insert(key, parent);
    //         self.children.entry(parent).or_default().push(key);
    //     }

    //     key
    // }

    // pub fn remove<F: FnMut(NodeId, Option<Key>)>(&mut self, key: NodeId, mut op: F) {
    //     let mut queue: VecDeque<_> = [key].into();

    //     while let Some(key) = queue.pop_front() {
    //         let k = self.nodes.remove(key).flatten();

    //         op(key, k);

    //         if let Some(parent) = self.parents.remove(&key) {
    //             if let Some(children) = self.children.get_mut(&parent) {
    //                 children.retain(|id| *id != key);
    //             }
    //         }

    //         if let Some(children) = self.children.remove(&key) {
    //             queue.extend(children);
    //         }
    //     }
    // }

    // pub fn get(&self, key: NodeId) -> Option<Key> {
    //     self.nodes.get(key).copied().flatten()
    // }

    // pub fn set(&mut self, id: NodeId, key: Key) {
    //     if let Some(node) = self.nodes.get_mut(id) {
    //         *node = Some(key);
    //     }
    // }

    // pub fn parent(&self, key: NodeId) -> Option<NodeId> {
    //     self.parents.get(&key).copied()
    // }
}

#[derive(Debug)]
pub struct TaskHandle<T>(ManuallyDrop<game_tasks::Task<T>>);

impl<T> Drop for TaskHandle<T> {
    fn drop(&mut self) {
        let task = unsafe { ManuallyDrop::take(&mut self.0) };
        task.cancel_now();
    }
}
