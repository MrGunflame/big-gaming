use std::any::Any;
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::fmt::{self, Debug, Formatter};
use std::marker::PhantomData;
use std::mem::ManuallyDrop;
use std::ptr::NonNull;
use std::rc::Rc;
use std::sync::Arc;

use game_common::collections::arena::{Arena, Key};
use game_input::keyboard::KeyboardInput;
use game_render::camera::RenderTarget;
use game_tracing::trace_span;
use glam::UVec2;
use parking_lot::Mutex;

use crate::layout::LayoutTree;
use crate::primitive::Primitive;
use crate::render::Text;
use crate::style::{Color, Style};
use crate::{widgets, WindowProperties};

pub trait Widget: Sized + 'static {
    type Message;

    fn create(&mut self, ctx: &Context<Self>) {}

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        false
    }

    fn render(&self, ctx: &Context<Self>) -> View;
}

trait RawWidget: 'static {
    fn update(&mut self, ctx: RawContext, msg: Box<dyn Any>) -> bool;

    fn render(&self, ctx: RawContext) -> View;
}

impl<T> RawWidget for T
where
    T: Widget,
{
    fn update(&mut self, ctx: RawContext, msg: Box<dyn Any>) -> bool {
        let ctx = Context {
            raw: ctx,
            _m: PhantomData,
        };
        T::update(self, &ctx, *msg.downcast().unwrap())
    }

    fn render(&self, ctx: RawContext) -> View {
        let ctx = Context {
            raw: ctx,
            _m: PhantomData,
        };
        T::render(self, &ctx)
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
        let mut rt = self.inner.lock();
        assert!(rt.windows.contains_key(&window));

        let document = Document::new(window, root);
        let key = rt.documents.insert(document);

        let window = rt.windows.get_mut(&window).unwrap();
        window.documents.push(DocumentId(key));

        DocumentId(key)
    }

    pub fn unmount(&self, document: DocumentId) {}
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
}

#[derive(Debug)]
pub(crate) struct Document {
    window: RenderTarget,
    pub(crate) tree: LayoutTree,
    nodes: Arena<Node>,
    events: Rc<RefCell<VecDeque<Event>>>,
    // Parent ==> Children
    children: HashMap<Key, Vec<Key>>,
    // Child ==> Parent
    parents: HashMap<Key, Key>,
}

impl Document {
    fn new<T>(window: RenderTarget, widget: T) -> Self
    where
        T: Widget,
    {
        let mut nodes = Arena::new();
        let key = nodes.insert(Node {
            node: Rc::new(RefCell::new(widget)),
            layout_key: None,
        });

        let mut this = Self {
            window,
            tree: LayoutTree::new(),
            nodes,
            children: HashMap::new(),
            events: Default::default(),
            parents: HashMap::new(),
        };

        this.layout_root(key);
        this
    }

    fn layout_root(&mut self, key: Key) {
        let root = self.nodes.get_mut(key).unwrap();

        let ctx = RawContext {
            events: self.events.clone(),
            key,
        };

        let mut view_queue = VecDeque::new();
        view_queue.push_back(root.node.borrow_mut().render(ctx));

        while let Some(view) = view_queue.pop_front() {
            match view {
                View::Primitive(primitive) => {
                    let root = self.nodes.get_mut(key).unwrap();
                    let key = self.tree.push(None, primitive);
                    root.layout_key = Some(key);
                }
                View::Component(component) => {
                    let child_key = self.nodes.insert(Node {
                        node: component,
                        layout_key: None,
                    });

                    self.parents.insert(child_key, key);
                    self.children.insert(child_key, Vec::new());
                    self.layout_root(child_key);
                }
                View::List(list) => view_queue.extend(list),
                View::Container(elem, children) => {
                    todo!()
                }
            }
        }
    }

    fn update(&mut self) {
        while let Some(event) = (|| self.events.borrow_mut().pop_front())() {
            match event {
                Event::SendMessage(key, msg) => {
                    let Some(node) = self.nodes.get_mut(key) else {
                        continue;
                    };

                    let ctx = RawContext {
                        events: self.events.clone(),
                        key,
                    };

                    if node.node.borrow_mut().update(ctx, msg) {
                        self.remove_children(key);
                        self.layout_root(key);
                    }
                }
            }
        }
    }

    fn remove_children(&mut self, key: Key) {
        let mut despawn_queue = Vec::new();

        if let Some(c) = self.children.remove(&key) {
            despawn_queue.extend(c);
        }

        while let Some(key) = despawn_queue.pop() {
            self.parents.remove(&key);

            if let Some(c) = self.children.remove(&key) {
                despawn_queue.extend(c);
            }
        }
    }
}

struct Node {
    node: Rc<RefCell<dyn RawWidget>>,
    layout_key: Option<crate::layout::Key>,
}

impl Debug for Node {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Node").finish_non_exhaustive()
    }
}

fn run(widget: impl Widget) {
    let mut nodes = Arena::new();

    let key = nodes.insert(Box::new(widget) as Box<dyn RawWidget>);

    let ctx = RawContext {
        key,
        events: Rc::new(RefCell::new(VecDeque::new())),
    };

    nodes.get(key).unwrap().render(ctx);
}

#[derive(Clone)]
pub enum View {
    Component(Rc<RefCell<dyn RawWidget>>),
    Primitive(Primitive),
    List(Vec<View>),
    // root and children
    Container(Primitive, Box<View>),
}

impl<T> From<T> for View
where
    T: Widget,
{
    fn from(value: T) -> Self {
        Self::Component(Rc::new(RefCell::new(value)))
    }
}

impl<const N: usize, T> From<[T; N]> for View
where
    T: Widget,
{
    fn from(value: [T; N]) -> Self {
        let mut list = Vec::with_capacity(N);
        for val in value {
            list.push(Self::Component(Rc::new(RefCell::new(val))));
        }

        Self::List(list)
    }
}

// impl<I, T> From<I> for View
// where
//     I: IntoIterator<Item = T>,
//     T: Widget,
// {
//     fn from(value: I) -> Self {
//         Self::List(
//             value
//                 .into_iter()
//                 .map(|v| Self::Component(Box::new(v)))
//                 .collect(),
//         )
//     }
// }

struct RawContext {
    key: Key,
    events: Rc<RefCell<VecDeque<Event>>>,
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
        F: Fn(I) -> T::Message + 'static,
    {
        let key = self.raw.key;
        let events = self.raw.events.clone();

        Callback(Rc::new(move |value| {
            let msg = f(value);

            events
                .borrow_mut()
                .push_back(Event::SendMessage(key, Box::new(msg)));
        }))
    }

    pub fn on_keyboard_input<F>(&self, f: F) -> EventHandler
    where
        F: Fn(KeyboardInput) -> T::Message + 'static,
    {
        todo!()
    }
}

struct EventHandler(Rc<dyn Fn()>);

pub struct Callback<T>(Rc<dyn Fn(T)>);

impl<T> Callback<T> {
    pub fn call(&self, value: T) {
        (self.0)(value);
    }
}

enum Event {
    SendMessage(Key, Box<dyn Any>),
}

impl Debug for Event {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Event").finish_non_exhaustive()
    }
}
