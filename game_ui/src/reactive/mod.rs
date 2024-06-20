use std::alloc::Layout;
use std::any::TypeId;
use std::collections::HashMap;
use std::mem::{ManuallyDrop, MaybeUninit};
use std::ptr::NonNull;
use std::sync::Arc;

use game_common::collections::arena::{self, Arena};
use game_render::camera::RenderTarget;
use game_window::windows::WindowId;
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
        node: Node,
    ) -> Option<NodeId> {
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

        let id = NodeId(rt.nodes.insert(node));

        if let Some(parent) = parent {
            rt.hierarchy.children.entry(parent).or_default().push(id);
            rt.hierarchy.parents.insert(id, parent);
        }

        document.layout_node_map.insert(id, node_key);

        Some(id)
    }

    pub fn create_document(&self, window: RenderTarget) -> Option<DocumentId> {
        let rt = &mut *self.inner.lock();
        let window = rt.windows.get_mut(&window)?;

        let doc = DocumentId(rt.documents.insert(Document {
            root: Vec::new(),
            layout: LayoutTree::new(),
            layout_node_map: HashMap::new(),
        }));

        window.documents.push(doc);
        Some(doc)
    }

    pub fn remove(&self, node: NodeId) {}

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

    pub(crate) fn destroy_window(&self, id: RenderTarget) {
        let mut rt = self.inner.lock();
        rt.windows.remove(&id);
    }
}

#[derive(Debug)]
pub(crate) struct RuntimeInner {
    pub(crate) windows: HashMap<RenderTarget, Window>,
    pub(crate) documents: Arena<Document>,
    nodes: Arena<Node>,
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
    root: Vec<NodeId>,
    pub(crate) layout: LayoutTree,
    layout_node_map: HashMap<NodeId, layout::Key>,
}

#[derive(Debug)]
pub struct Node {
    primitive: Primitive,
    event_handlers: EventHandlers,
}

impl Node {
    pub fn new(primitive: Primitive) -> Self {
        Self {
            primitive,
            event_handlers: EventHandlers::default(),
        }
    }

    pub fn register<E, F>(&mut self, handler: F)
    where
        F: FnMut(E) + Send + Sync + 'static,
        E: Event2,
    {
        self.event_handlers.register(handler);
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
    handler: ManuallyDrop<Box<dyn FnMut(E) + Send + Sync + 'static>>,
}

impl<E> RawEventHandler<E> {
    const LAYOUT: Layout = Layout::new::<Self>();

    unsafe fn call(ptr: NonNull<()>, event: *const ()) {
        unsafe {
            let this = ptr.cast::<Self>().as_mut();
            let event = event.cast::<E>().read();

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
        F: FnMut(E) + Send + Sync + 'static,
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

    unsafe fn call<E>(&mut self, event: E) {
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

#[derive(Debug, Default)]
struct EventHandlers {
    // TypeId::of<E> -> Box<dyn FnMut(E)>
    map: HashMap<TypeId, EventHandlerPtr>,
}

impl EventHandlers {
    fn call<E>(&mut self, event: E)
    where
        E: Event2,
    {
        if let Some(handler) = self.map.get_mut(&TypeId::of::<E>()) {
            unsafe {
                handler.call(event);
            }
        }
    }

    fn register<E, F>(&mut self, handler: F)
    where
        F: FnMut(E) + Send + Sync + 'static,
        E: Event2,
    {
        self.map
            .insert(TypeId::of::<E>(), EventHandlerPtr::new(handler));
    }
}

pub trait Event2: Sized + Send + Sync + 'static {}

pub struct Context<E> {
    pub event: E,
    parent: Option<NodeId>,
    window: WindowId,
    document: DocumentId,
    runtime: Runtime,
}

impl<E> Context<E> {
    pub fn parent(&self) -> Option<NodeId> {
        self.parent
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct NodeId(arena::Key);

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
