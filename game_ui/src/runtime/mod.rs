pub mod events;
pub mod reactive;

use std::any::{Any, TypeId};
use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::mem::ManuallyDrop;
use std::sync::Arc;

use events::{Event, EventHandlerId, EventHandlers, NodeDestroyed};
use game_common::collections::arena::{Arena, Key};
use game_render::camera::RenderTarget;
use game_tasks::TaskPool;
use game_tracing::trace_span;
use game_window::cursor::Cursor;
use game_window::windows::WindowId;
use glam::UVec2;
use parking_lot::Mutex;
use reactive::ReactiveRuntime;

use crate::clipboard::Clipboard;
use crate::layout::{self, LayoutTree};
use crate::primitive::Primitive;
use crate::render::Rect;
use crate::WindowProperties;

#[derive(Clone, Debug)]
pub struct Runtime {
    pub(crate) inner: Arc<Mutex<RuntimeInner>>,
    pub(crate) cursor: Arc<Mutex<Option<Arc<Cursor>>>>,
    pub(crate) pool: Arc<TaskPool>,
    clipboard: Arc<Mutex<Clipboard>>,
    reactive: ReactiveRuntime,
}

impl Runtime {
    /// Creates a new `Runtime`.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(RuntimeInner {
                windows: HashMap::new(),
                documents: Arena::new(),
                event_handlers: EventHandlers::new(),
            })),
            cursor: Arc::new(Mutex::new(None)),
            pool: Arc::new(TaskPool::new(1)),
            clipboard: Arc::new(Mutex::new(Clipboard::new())),
            reactive: ReactiveRuntime::new(),
        }
    }

    pub fn reactive(&self) -> &ReactiveRuntime {
        &self.reactive
    }

    /// Returns access to the windows managed by this `Runtime`.
    #[inline]
    pub(crate) fn windows(&self) -> RuntimeWindows<'_> {
        RuntimeWindows { runtime: self }
    }

    /// Dispatches a new event to this `Runtime`.
    pub(crate) fn send_event<E>(&self, window: RenderTarget, event: E)
    where
        E: Event + Clone,
    {
        let _span = trace_span!("Runtime::send_event").entered();

        let rt = self.inner.lock();
        let mut handlers = Vec::new();

        for document in rt.documents.values() {
            if document.window != window {
                continue;
            }

            for id in &document.node_event_handlers {
                if let Some(handler) = rt.event_handlers.get_by_id::<E>(*id) {
                    handlers.push(handler);
                }
            }

            for id in &document.global_event_handlers {
                if let Some(handler) = rt.event_handlers.get_by_id::<E>(*id) {
                    handlers.push(handler);
                }
            }
        }

        drop(rt);

        // Call event handlers bottom-up.
        for handler in handlers.into_iter().rev() {
            handler.call(event.clone());
        }
    }

    /// Returns a list of documents in this `Runtime`.
    pub fn documents(&self, window: RenderTarget) -> Vec<DocumentId> {
        let _span = trace_span!("Runtime::documents").entered();

        let rt = self.inner.lock();
        rt.windows
            .get(&window)
            .map(|w| w.documents.clone())
            .unwrap_or(Vec::new())
    }

    /// Returns a new [`Context`] to the root of a given [`DocumentId`].
    pub fn root_context(&self, document: DocumentId) -> Context {
        Context {
            runtime: self.clone(),
            document,
            node: None,
        }
    }

    /// Creates a new document on the given [`RenderTarget`]. Returns `None` if no such
    /// [`RenderTarget`] exists.
    pub fn create_document(&self, target: RenderTarget) -> Option<DocumentId> {
        let _span = trace_span!("Runtime::create_document").entered();

        let rt = &mut *self.inner.lock();
        let window = rt.windows.get_mut(&target)?;

        let doc = DocumentId(rt.documents.insert(Document {
            window: target,
            nodes: Arena::new(),
            tree: LayoutTree::new(),
            children: HashMap::new(),
            parents: HashMap::new(),
            root: HashSet::new(),
            node_event_handlers: HashSet::new(),
            global_event_handlers: Vec::new(),
            type_map: HashMap::new(),
        }));

        window.documents.push(doc);
        Some(doc)
    }

    /// Destroys the document with the given `document_id`.
    pub fn destroy_document(&self, document_id: DocumentId) {
        let _span = trace_span!("Runtime::destroy_document").entered();

        // Before we can remove the document we must destroy
        // all nodes in the document.
        let rt = self.inner.lock();
        let Some(document) = rt.documents.get(document_id.0) else {
            return;
        };

        let root = document.root.clone();

        drop(rt);
        for node in root {
            self.remove(document_id, node);
        }

        let mut rt = self.inner.lock();
        let document = rt.documents.remove(document_id.0).unwrap();

        for id in document.global_event_handlers {
            rt.event_handlers.remove(id);
        }

        let window = rt.windows.get_mut(&document.window).unwrap();
        window.documents.retain(|id| *id != document_id);
    }

    fn remove(&self, document: DocumentId, node: NodeId) {
        let _span = trace_span!("Runtime::remove").entered();

        let mut node_destroyed_handlers = Vec::new();

        {
            let rt = &mut *self.inner.lock();
            let document = rt.documents.get_mut(document.0).unwrap();

            if !document.nodes.contains_key(node.0) {
                return;
            }

            document.root.remove(&node);

            let mut nodes_removed = Vec::new();
            let mut queue = vec![node];

            while let Some(key) = queue.pop() {
                let node = document
                    .nodes
                    .remove(key.0)
                    .expect("runtime tree corrupted");

                nodes_removed.push(node);

                if let Some(children) = document.children.remove(&key) {
                    queue.extend(children);
                }

                if let Some(parent) = document.parents.remove(&key) {
                    if let Some(children) = document.children.get_mut(&parent) {
                        children.retain(|c| *c != key);
                    }
                }
            }

            for node in nodes_removed {
                document.tree.remove(node.layout_key);

                for id in node.event_handlers {
                    if let Some(handler) = rt.event_handlers.get_by_id::<NodeDestroyed>(id) {
                        node_destroyed_handlers.push(handler);
                    }

                    rt.event_handlers.remove(id);
                }
            }
        }

        // Destroy the deepest nodes first.
        for handler in node_destroyed_handlers.into_iter() {
            handler.call(NodeDestroyed);
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct RuntimeWindows<'a> {
    runtime: &'a Runtime,
}

impl<'a> RuntimeWindows<'a> {
    /// Creates a new window with the given [`WindowProperties`].
    pub fn create(&self, window: RenderTarget, props: WindowProperties) {
        let _span = trace_span!("RuntimeWindows::create").entered();

        let mut rt = self.runtime.inner.lock();
        rt.windows.insert(
            window,
            Window {
                documents: Vec::new(),
                size: props.size,
                scale_factor: props.scale_factor,
            },
        );

        self.runtime.clipboard.lock().create(props.state);
    }

    /// Destroys the window with the given [`RenderTarget`]. Does nothing if no window with the
    /// given [`RenderTarget`] exists.
    pub fn destroy(&self, target: RenderTarget) {
        let _span = trace_span!("RuntimeWindows::destroy").entered();

        if let Some(window) = target.as_window() {
            self.runtime.clipboard.lock().destroy(*window);
        }

        let rt = self.runtime.inner.lock();
        let Some(window) = rt.windows.get(&target) else {
            return;
        };

        let documents = window.documents.clone();

        drop(rt);
        for document in documents {
            self.runtime.destroy_document(document);
        }

        let mut rt = self.runtime.inner.lock();
        rt.windows.remove(&target).unwrap();
    }

    /// Updates the size of window with the given [`RenderTarget`].
    pub fn update_size(&self, window: RenderTarget, size: UVec2) {
        let _span = trace_span!("RuntimeWindows::update_size").entered();

        let rt = &mut *self.runtime.inner.lock();
        let Some(window) = rt.windows.get_mut(&window) else {
            return;
        };

        window.size = size;

        for document_id in &window.documents {
            let document = rt.documents.get_mut(document_id.0).unwrap();
            document.tree.resize(size);
        }
    }

    /// Updates the scale factor of the window with the given [`RenderTarget`].
    pub fn update_scale_factor(&self, window: RenderTarget, scale_factor: f64) {
        let _span = trace_span!("RuntimeWindows::update_scale_factor").entered();

        let rt = &mut *self.runtime.inner.lock();
        let Some(window) = rt.windows.get_mut(&window) else {
            return;
        };

        window.scale_factor = scale_factor;

        for document_id in &window.documents {
            let document = rt.documents.get_mut(document_id.0).unwrap();
            document.tree.set_scale_factor(scale_factor);
        }
    }
}

#[derive(Debug)]
pub(crate) struct RuntimeInner {
    windows: HashMap<RenderTarget, Window>,
    pub(crate) documents: Arena<Document>,
    event_handlers: EventHandlers,
}

#[derive(Debug)]
struct Window {
    documents: Vec<DocumentId>,
    size: UVec2,
    scale_factor: f64,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct DocumentId(Key);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct NodeId(Key);

#[derive(Debug)]
pub(crate) struct Document {
    pub(crate) window: RenderTarget,
    pub(crate) tree: LayoutTree,

    nodes: Arena<Node>,
    parents: HashMap<NodeId, NodeId>,
    children: HashMap<NodeId, Vec<NodeId>>,
    root: HashSet<NodeId>,

    node_event_handlers: HashSet<EventHandlerId>,
    global_event_handlers: Vec<EventHandlerId>,
    type_map: HashMap<TypeId, Arc<dyn Any + Send + Sync + 'static>>,
}

#[derive(Debug)]
pub struct Node {
    layout_key: layout::Key,
    event_handlers: Vec<EventHandlerId>,
}

/// A context to a node somewhere in the widget tree of a document.
#[derive(Clone, Debug)]
pub struct Context {
    runtime: Runtime,
    document: DocumentId,
    node: Option<NodeId>,
}

impl Context {
    /// Returns the [`NodeId`] referenced by this `Context`.
    pub fn node(&self) -> Option<NodeId> {
        self.node
    }

    /// Returns a reference to the underlying [`Runtime`] that is managing this `Context`.
    pub fn runtime(&self) -> &Runtime {
        &self.runtime
    }

    /// Returns access to the system clipboard.
    #[inline]
    pub fn clipboard(&self) -> ClipboardRef<'_> {
        ClipboardRef {
            rt: &self.runtime,
            document: self.document,
        }
    }

    /// Returns access to the cursor state.
    #[inline]
    pub fn cursor(&self) -> CursorRef<'_> {
        CursorRef { rt: &self.runtime }
    }

    /// Appends a new [`Primitive`] as the last child of this node. Returns the `Context` of the
    /// newly appended node.
    ///
    /// # Panics
    ///
    /// Panics if the node references by `self` has been destroyed.
    pub fn append(&self, primitive: Primitive) -> Context {
        let mut rt = self.runtime.inner.lock();
        let document = rt.documents.get_mut(self.document.0).unwrap();

        let parent = self.node.map(|node| {
            let node = document
                .nodes
                .get(node.0)
                .expect("attempted to append to a non-existant node");
            node.layout_key
        });

        let layout_key = document.tree.push(parent, primitive);
        let key = NodeId(document.nodes.insert(Node {
            layout_key,
            event_handlers: Vec::new(),
        }));

        document.children.insert(key, Vec::new());

        if let Some(parent) = self.node {
            document.parents.insert(key, parent);
            document.children.get_mut(&parent).unwrap().push(key);
        } else {
            document.root.insert(key);
        }

        Self {
            runtime: self.runtime.clone(),
            document: self.document,
            node: Some(key),
        }
    }

    /// Removes the given `node`. Does nothing if the given `node` does not exist.
    pub fn remove(&self, node: NodeId) {
        self.runtime.remove(self.document, node);
    }

    /// Removes the node referenced by `self`.
    ///
    /// Note that this means some functions on this `Context` become invalid and may result in
    /// panics if used.
    pub fn remove_self(&self) {
        if let Some(node) = self.node {
            self.remove(node);
        }
    }

    /// Removes all children of the current node. Does nothing if the current node does not exist.
    pub fn clear_children(&self) {
        let _span = trace_span!("Context::clear_children").entered();

        let mut rt = self.runtime.inner.lock();
        let document = rt.documents.get_mut(self.document.0).unwrap();

        let Some(node) = self.node else {
            return;
        };

        let Some(children) = document.children.get(&node).cloned() else {
            return;
        };

        drop(rt);

        for node in children {
            self.runtime.remove(self.document, node);
        }
    }

    /// Returns the layout of the given `node`.
    ///
    /// The returned [`Rect`] contains the area that the given `node` will use for painting.
    /// Returns `None` if the given `node` does not exist.
    pub fn layout(&self, node: NodeId) -> Option<Rect> {
        let mut rt = self.runtime.inner.lock();
        let document = rt.documents.get_mut(self.document.0)?;
        let node = document.nodes.get(node.0)?;
        document.tree.layout(node.layout_key).map(|layout| Rect {
            min: layout.position,
            max: UVec2 {
                x: layout.position.x + layout.width,
                y: layout.position.y + layout.height,
            },
        })
    }

    /// Returns a reference to the underlying document of this `Context`.
    pub fn document(&self) -> DocumentRef<'_> {
        DocumentRef {
            rt: &self.runtime,
            id: self.document,
        }
    }

    /// Spawns a [`Future`] and returns a [`TaskHandle`].
    ///
    /// If the returned [`TaskHandle`] is dropped, the future is cancelled if it did not already
    /// complete.
    pub fn spawn_task<F>(&self, future: F) -> TaskHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let handle = self.runtime.pool.spawn(future);
        TaskHandle(ManuallyDrop::new(handle))
    }
}

/// Access to a document.
pub struct DocumentRef<'a> {
    rt: &'a Runtime,
    id: DocumentId,
}

impl<'a> DocumentRef<'a> {
    /// Registers an document-wide event handler.
    pub fn register<E, F>(&self, handler: F)
    where
        F: FnMut(E) + Send + 'static,
        E: Event,
    {
        self.register_on_self(None, handler);
    }

    /// Registers a document-wide event handler attached to a `parent` node.
    ///
    /// If the `parent` node is destroyed, the event handler is removed.
    pub fn register_with_parent<E, F>(&self, parent: NodeId, handler: F)
    where
        F: FnMut(E) + Send + 'static,
        E: Event,
    {
        self.register_on_self(Some(parent), handler);
    }

    fn register_on_self<E, F>(&self, parent: Option<NodeId>, handler: F)
    where
        F: FnMut(E) + Send + 'static,
        E: Event,
    {
        let mut rt = self.rt.inner.lock();

        let id = rt.event_handlers.insert(handler);
        let document = rt.documents.get_mut(self.id.0).unwrap();

        if let Some(parent) = parent {
            let node = document.nodes.get_mut(parent.0).unwrap();
            node.event_handlers.push(id);
            document.node_event_handlers.insert(id);
        } else {
            if TypeId::of::<E>() == TypeId::of::<NodeDestroyed>() {
                panic!("NodeDestroyed event handler must be attached to a node");
            }

            document.global_event_handlers.push(id);
        }
    }

    pub fn get<T>(&self) -> Option<Arc<T>>
    where
        T: Send + Sync + 'static,
    {
        let _span = trace_span!("DocumentRef::get").entered();

        let mut rt = self.rt.inner.lock();
        let document = rt.documents.get_mut(self.id.0)?;
        document
            .type_map
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

/// Access to the cursor.
#[derive(Clone, Debug)]
pub struct CursorRef<'a> {
    rt: &'a Runtime,
}

impl<'a> CursorRef<'a> {
    /// Returns the current position of the cursor. Returns `None` if the cursor is not in the
    /// current window.
    pub fn position(&self) -> Option<UVec2> {
        match &*self.rt.cursor.lock() {
            Some(cursor) => Some(cursor.position().as_uvec2()),
            None => None,
        }
    }
}

#[derive(Debug)]
pub struct TaskHandle<T>(ManuallyDrop<game_tasks::Task<T>>);

impl<T> Drop for TaskHandle<T> {
    fn drop(&mut self) {
        let task = unsafe { ManuallyDrop::take(&mut self.0) };
        task.cancel_now();
    }
}

/// Access to the system clipboard.
#[derive(Clone, Debug)]
pub struct ClipboardRef<'a> {
    rt: &'a Runtime,
    document: DocumentId,
}

impl<'a> ClipboardRef<'a> {
    /// Sets the current value of the system clipboard.
    pub fn set(&self, value: &str) {
        let Some(window) = self.window() else {
            return;
        };

        self.rt.clipboard.lock().set(window, value);
    }

    /// Returns the given value of the system clipboard, if any.
    pub fn get(&self) -> Option<String> {
        let Some(window) = self.window() else {
            return None;
        };

        self.rt.clipboard.lock().get(window)
    }

    fn window(&self) -> Option<WindowId> {
        let rt = self.rt.inner.lock();
        let Some(document) = rt.documents.get(self.document.0) else {
            return None;
        };

        // Images have no associated clipboard.
        document.window.as_window().copied()
    }
}
