use std::alloc::Layout;
use std::any::TypeId;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::mem::{ManuallyDrop, MaybeUninit};
use std::ptr::NonNull;
use std::sync::Arc;

use game_common::collections::arena::{self, Arena};
use game_render::camera::RenderTarget;
use glam::UVec2;
use parking_lot::Mutex;

use crate::layout::{self, LayoutTree};
use crate::primitive::Primitive;

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
                nodes: Arena::new(),
                hierarchy: NodeHierarchy::default(),
            })),
        }
    }

    pub fn append(
        &self,
        document: DocumentId,
        parent: Option<NodeId>,
        mut node: Node,
    ) -> Option<NodeId> {
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

        Some(id)
    }

    pub fn create_document(&self, window: RenderTarget) -> Option<DocumentId> {
        let rt = &mut *self.inner.lock();
        let window = rt.windows.get_mut(&window)?;

        let doc = DocumentId(rt.documents.insert(Document {
            layout: LayoutTree::new(),
            layout_node_map: HashMap::new(),
            layout_node_map2: HashMap::new(),
            event_handlers: EventHandlers::default(),
        }));

        window.documents.push(doc);
        Some(doc)
    }

    pub fn clear_children(&self, node: NodeId) {
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
        let rt = &mut *self.inner.lock();

        let mut children = vec![node];
        while let Some(node_id) = children.pop() {
            let Some(node) = rt.nodes.remove(node_id.0) else {
                continue;
            };

            if let Some(parent) = rt.hierarchy.parents.remove(&node_id) {
                rt.hierarchy
                    .children
                    .get_mut(&parent)
                    .unwrap()
                    .retain(|child| *child != node_id);
            }

            if let Some(c) = rt.hierarchy.children.remove(&node_id) {
                children.extend(c);
            }

            let doc = rt.documents.get_mut(node.document.unwrap().0).unwrap();
            let key = doc.layout_node_map.remove(&node_id).unwrap();
            doc.layout_node_map2.remove(&key).unwrap();
            doc.layout.remove(key);
        }
    }

    pub(crate) fn create_window(&self, id: RenderTarget, size: UVec2) {
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
        let rt = &mut *self.inner.lock();
        let window = rt.windows.get_mut(&id).unwrap();
        window.size = size;

        for doc in &window.documents {
            rt.documents.get_mut(doc.0).unwrap().layout.resize(size);
        }
    }

    pub(crate) fn destroy_window(&self, id: RenderTarget) {
        let mut rt = self.inner.lock();
        rt.windows.remove(&id);
    }

    pub fn root_context(&self, document: DocumentId) -> Context<()> {
        Context {
            event: (),
            node: None,
            document,
            runtime: self.clone(),
        }
    }
}

#[derive(Debug)]
pub(crate) struct RuntimeInner {
    pub(crate) windows: HashMap<RenderTarget, Window>,
    pub(crate) documents: Arena<Document>,
    pub(crate) nodes: Arena<Node>,
    hierarchy: NodeHierarchy,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct DocumentId(pub(crate) arena::Key);

#[derive(Debug)]
pub struct Window {
    pub documents: Vec<DocumentId>,
    size: UVec2,
}

#[derive(Debug)]
pub struct Document {
    event_handlers: EventHandlers,
    pub(crate) layout: LayoutTree,
    pub(crate) layout_node_map: HashMap<NodeId, layout::Key>,
    pub(crate) layout_node_map2: HashMap<layout::Key, NodeId>,
}

impl Document {
    pub fn register<E, F>(&mut self, handler: F)
    where
        F: FnMut(Context<E>) + Send + Sync + 'static,
        E: Event,
    {
        self.event_handlers.register(handler);
    }

    pub(crate) fn get<E>(&self) -> Option<EventHandler<E>>
    where
        E: Event,
    {
        self.event_handlers.get()
    }
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

    pub fn register<E, F>(&mut self, handler: F)
    where
        F: FnMut(Context<E>) + Send + Sync + 'static,
        E: Event,
    {
        self.event_handlers.register(handler);
    }

    // pub fn send<E>(&mut self, event: E)
    // where
    //     E: Event,
    // {
    //     self.event_handlers.call(event);
    // }

    pub(crate) fn get<E>(&self) -> Option<EventHandler<E>>
    where
        E: Event,
    {
        self.event_handlers.get()
    }
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

pub(crate) struct EventHandler<E> {
    ptr: Arc<Mutex<EventHandlerPtr>>,
    _marker: PhantomData<fn(E)>,
}

impl<E> EventHandler<E>
where
    E: Event,
{
    pub fn call(&self, event: Context<E>) {
        unsafe {
            self.ptr.lock().call(event);
        }
    }
}

#[derive(Debug, Default)]
struct EventHandlers {
    // TypeId::of<E> -> Box<dyn FnMut(E)>
    map: HashMap<TypeId, Arc<Mutex<EventHandlerPtr>>>,
}

impl EventHandlers {
    fn get<E>(&self) -> Option<EventHandler<E>>
    where
        E: Event,
    {
        self.map.get(&TypeId::of::<E>()).map(|ptr| EventHandler {
            ptr: ptr.clone(),
            _marker: PhantomData,
        })
    }

    fn register<E, F>(&mut self, handler: F)
    where
        F: FnMut(Context<E>) + Send + Sync + 'static,
        E: Event,
    {
        self.map.insert(
            TypeId::of::<E>(),
            Arc::new(Mutex::new(EventHandlerPtr::new(handler))),
        );
    }
}

unsafe impl Send for EventHandlers {}
unsafe impl Sync for EventHandlers {}

pub trait Event: Sized + Send + Sync + 'static {}

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
        let mut rt = self.rt.inner.lock();
        rt.documents.get_mut(self.id.0).unwrap().register(handler);
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
