pub mod events;
pub mod reactive;

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::future::Future;
use std::mem::ManuallyDrop;
use std::sync::Arc;

use events::{Event, EventHandlerId, EventHandlers, NodeDestroyed};
use game_common::collections::arena::{Arena, Key};
use game_render::camera::RenderTarget;
use game_tasks::TaskPool;
use game_tracing::trace_span;
use game_window::cursor::Cursor;
use glam::{UVec2, Vec2};
use parking_lot::Mutex;

use crate::layout::{self, LayoutTree};
use crate::primitive::Primitive;
use crate::render::Rect;
use crate::WindowProperties;

#[derive(Clone, Debug)]
pub struct Runtime {
    pub(crate) inner: Arc<Mutex<RuntimeInner>>,
    pub(crate) cursor: Arc<Mutex<Option<Arc<Cursor>>>>,
    pub(crate) pool: Arc<TaskPool>,
}

impl Runtime {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(RuntimeInner {
                windows: HashMap::new(),
                documents: Arena::new(),
                event_handlers: EventHandlers::new(),
            })),
            cursor: Arc::new(Mutex::new(None)),
            pool: Arc::new(TaskPool::new(1)),
        }
    }

    #[inline]
    pub(crate) fn windows(&self) -> RuntimeWindows<'_> {
        RuntimeWindows { runtime: self }
    }

    pub(crate) fn send_event<E>(&self, event: E)
    where
        E: Event + Clone,
    {
        let rt = self.inner.lock();
        let Some(handlers) = rt.event_handlers.get::<E>() else {
            return;
        };

        drop(rt);

        // Call event handlers bottom-up.
        for handler in handlers.into_iter().rev() {
            handler.call(event.clone());
        }
    }

    pub fn root_context(&self, document: DocumentId) -> Context {
        Context {
            runtime: self.clone(),
            document,
            node: None,
        }
    }

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
            global_event_handlers: Vec::new(),
            type_map: HashMap::new(),
        }));

        window.documents.push(doc);
        Some(doc)
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

        for handler in node_destroyed_handlers.into_iter().rev() {
            handler.call(NodeDestroyed);
        }
    }

    pub fn clipboard_set(&self, value: &str) {
        todo!()
    }

    pub fn clipboard_get(&self) -> Option<String> {
        todo!()
    }
}

#[derive(Copy, Clone, Debug)]
pub struct RuntimeWindows<'a> {
    runtime: &'a Runtime,
}

impl<'a> RuntimeWindows<'a> {
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
    }

    pub fn destroy(&self, window: RenderTarget) {
        let _span = trace_span!("RuntimeWindows::destroy").entered();

        let mut rt = self.runtime.inner.lock();
        let Some(window) = rt.windows.remove(&window) else {
            return;
        };
    }

    pub fn update_size(&self, window: RenderTarget, size: UVec2) {
        let _span = trace_span!("RuntimeWindows::update_size").entered();

        let mut rt = self.runtime.inner.lock();
        if let Some(window) = rt.windows.get_mut(&window) {
            window.size = size;
        }
    }

    pub fn update_scale_factor(&self, window: RenderTarget, scale_factor: f64) {
        let _span = trace_span!("RuntimeWindows::update_scale_factor").entered();

        let mut rt = self.runtime.inner.lock();
        if let Some(window) = rt.windows.get_mut(&window) {
            window.scale_factor = scale_factor;
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
    global_event_handlers: Vec<EventHandlerId>,
    type_map: HashMap<TypeId, Arc<dyn Any + Send + Sync + 'static>>,
}

#[derive(Debug)]
pub struct Node {
    layout_key: layout::Key,
    event_handlers: Vec<EventHandlerId>,
}

#[derive(Clone, Debug)]
pub struct Context {
    runtime: Runtime,
    document: DocumentId,
    node: Option<NodeId>,
}

impl Context {
    pub fn node(&self) -> Option<NodeId> {
        self.node
    }

    pub fn runtime(&self) -> &Runtime {
        &self.runtime
    }

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
        }

        Self {
            runtime: self.runtime.clone(),
            document: self.document,
            node: Some(key),
        }
    }

    pub fn remove(&self, node: NodeId) {
        self.runtime.remove(self.document, node);
    }

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

    // pub fn reactive(&self) -> &ReactiveContext {
    //     &self.runtime.reactive
    // }

    pub fn layout(&self, node: NodeId) -> Option<Rect> {
        let mut rt = self.runtime.inner.lock();
        let document = rt.documents.get_mut(self.document.0).unwrap();
        let node = document.nodes.get(node.0).unwrap();
        document.tree.layout(node.layout_key).map(|layout| Rect {
            min: layout.position,
            max: UVec2 {
                x: layout.position.x + layout.width,
                y: layout.position.y + layout.height,
            },
        })
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

    pub fn spawn_task<F>(&self, future: F) -> TaskHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let handle = self.runtime.pool.spawn(future);
        TaskHandle(ManuallyDrop::new(handle))
    }
}

pub struct DocumentRef<'a> {
    rt: &'a Runtime,
    id: DocumentId,
}

impl<'a> DocumentRef<'a> {
    pub fn register<E, F>(&self, handler: F)
    where
        F: FnMut(E) + Send + 'static,
        E: Event,
    {
        self.register_on_self(None, handler);
    }

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

pub struct CursorRef<'a> {
    rt: &'a Runtime,
}

#[derive(Debug)]
pub struct TaskHandle<T>(ManuallyDrop<game_tasks::Task<T>>);

impl<T> Drop for TaskHandle<T> {
    fn drop(&mut self) {
        let task = unsafe { ManuallyDrop::take(&mut self.0) };
        task.cancel_now();
    }
}
