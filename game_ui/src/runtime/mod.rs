pub mod events;
pub mod reactive;

use std::collections::HashMap;
use std::sync::Arc;

use game_common::collections::arena::{Arena, Key};
use game_render::camera::RenderTarget;
use game_tracing::trace_span;
use glam::UVec2;
use parking_lot::Mutex;
use reactive::ReactiveContext;

use crate::layout::{self, LayoutTree};
use crate::primitive::Primitive;
use crate::WindowProperties;

#[derive(Clone, Debug)]
pub struct Runtime {
    pub(crate) inner: Arc<Mutex<RuntimeInner>>,
    pub(crate) reactive: ReactiveContext,
}

impl Runtime {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(RuntimeInner {
                windows: HashMap::new(),
                documents: Arena::new(),
            })),
            reactive: ReactiveContext::new(),
        }
    }

    #[inline]
    pub(crate) fn windows(&self) -> RuntimeWindows<'_> {
        RuntimeWindows { runtime: self }
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
struct RuntimeInner {
    windows: HashMap<RenderTarget, Window>,
    pub(crate) documents: Arena<Document>,
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
pub struct NodeId(layout::Key);

#[derive(Debug)]
struct Document {
    pub(crate) window: RenderTarget,
    pub(crate) tree: LayoutTree,
}

pub struct Context {
    runtime: Runtime,
    document: DocumentId,
    node: Option<NodeId>,
}

impl Context {
    pub fn node(&self) -> Option<NodeId> {
        self.node
    }

    pub fn append(&self, primitive: Primitive) -> Context {
        let mut rt = self.runtime.inner.lock();
        let document = rt.documents.get_mut(self.document.0).unwrap();
        let key = document.tree.push(self.node.map(|v| v.0), primitive);

        Self {
            runtime: self.runtime.clone(),
            document: self.document,
            node: Some(NodeId(key)),
        }
    }

    pub fn remove(&self, node: NodeId) {
        let mut rt = self.runtime.inner.lock();
        let document = rt.documents.get_mut(self.document.0).unwrap();
        document.tree.remove(node.0);
    }

    pub fn clear_children(&self) {
        let mut rt = self.runtime.inner.lock();
        let document = rt.documents.get_mut(self.document.0).unwrap();

        let Some(node) = self.node else {
            // TODO: Clear all
            return;
        };

        if let Some(children) = document.tree.children(node.0) {
            for key in children.to_vec() {
                document.tree.remove(key);
            }
        }
    }

    pub fn reactive(&self) -> &ReactiveContext {
        &self.runtime.reactive
    }
}

// pub struct DocumentRef<'a> {
//     ctx: &'a Context,
// }
