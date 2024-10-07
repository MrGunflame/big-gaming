use std::alloc::Layout;
use std::any::TypeId;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::mem::{ManuallyDrop, MaybeUninit};
use std::ptr::NonNull;
use std::sync::Arc;

use game_common::collections::arena::{Arena, Key};
use game_input::keyboard::KeyboardInput;
use game_input::mouse::{MouseButtonInput, MouseWheel};
use game_window::events::CursorMoved;
use parking_lot::Mutex;

pub trait Event: Sized + 'static {}

impl Event for CursorMoved {}
impl Event for KeyboardInput {}
impl Event for MouseButtonInput {}
impl Event for MouseWheel {}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct NodeDestroyed;

impl Event for NodeDestroyed {}

#[derive(Debug)]
pub struct EventHandlers {
    handlers: Arena<Arc<Mutex<EventHandlerPtr>>>,
    map: HashMap<TypeId, Vec<EventHandlerId>>,
}

impl EventHandlers {
    pub fn new() -> Self {
        Self {
            handlers: Arena::new(),
            map: HashMap::new(),
        }
    }

    pub fn insert<E, F>(&mut self, handler: F) -> EventHandlerId
    where
        E: Event,
        F: FnMut(E) + Send + 'static,
    {
        let handler = Arc::new(Mutex::new(EventHandlerPtr::new(handler)));
        let key = EventHandlerId(self.handlers.insert(handler));

        self.map.entry(TypeId::of::<E>()).or_default().push(key);
        key
    }

    pub fn remove(&mut self, id: EventHandlerId) {
        self.handlers.remove(id.0);
        self.map.retain(|_, keys| {
            keys.retain(|key| *key != id);
            !keys.is_empty()
        });
    }

    pub fn get_by_id<E>(&self, id: EventHandlerId) -> Option<EventHandler<E>>
    where
        E: Event,
    {
        let handler = self.handlers.get(id.0)?;

        if handler.lock().type_id != TypeId::of::<E>() {
            None
        } else {
            Some(EventHandler {
                ptr: handler.clone(),
                _marker: PhantomData,
            })
        }
    }

    pub fn get<E>(&self) -> Option<Vec<EventHandler<E>>>
    where
        E: Event,
    {
        let keys = self.map.get(&TypeId::of::<E>())?;

        Some(
            keys.iter()
                .map(|key| {
                    let handler = self.handlers.get(key.0).unwrap();
                    EventHandler {
                        ptr: handler.clone(),
                        _marker: PhantomData,
                    }
                })
                .collect(),
        )
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct EventHandlerId(Key);

#[derive(Debug, Clone)]
pub(crate) struct EventHandler<E> {
    _marker: PhantomData<fn(E)>,
    ptr: Arc<Mutex<EventHandlerPtr>>,
}

impl<E> EventHandler<E> {
    pub fn call(&self, event: E)
    where
        E: Event,
    {
        unsafe {
            self.ptr.lock().call(event);
        }
    }
}

#[derive(Debug)]
struct Header {
    call: unsafe fn(NonNull<()>, NonNull<()>),
    drop: unsafe fn(NonNull<()>),
    layout: Layout,
}

#[derive(Debug)]
#[repr(C)]
struct RawEventHandler<E, F> {
    header: Header,
    handler: ManuallyDrop<F>,
    _marker: PhantomData<fn(E)>,
}

impl<E, F> RawEventHandler<E, F>
where
    F: FnMut(E) + 'static,
{
    const LAYOUT: Layout = Layout::new::<Self>();

    unsafe fn call(ptr: NonNull<()>, event: NonNull<()>) {
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

unsafe impl<E, F> Send for RawEventHandler<E, F> where F: Send {}

#[derive(Debug)]
struct EventHandlerPtr {
    ptr: NonNull<()>,
    type_id: TypeId,
}

impl EventHandlerPtr {
    fn new<E, F>(handler: F) -> Self
    where
        E: Event,
        F: FnMut(E) + Send + 'static,
    {
        let layout = RawEventHandler::<E, F>::LAYOUT;
        let ptr = unsafe {
            let ptr = std::alloc::alloc(layout);
            if ptr.is_null() {
                std::alloc::handle_alloc_error(layout);
            }

            ptr.cast::<RawEventHandler<E, F>>().write(RawEventHandler {
                header: Header {
                    call: RawEventHandler::<E, F>::call,
                    drop: RawEventHandler::<E, F>::drop,
                    layout: RawEventHandler::<E, F>::LAYOUT,
                },
                handler: ManuallyDrop::new(handler),
                _marker: PhantomData,
            });

            NonNull::new_unchecked(ptr).cast::<()>()
        };

        Self {
            ptr,
            type_id: TypeId::of::<E>(),
        }
    }

    unsafe fn call<E>(&mut self, event: E)
    where
        E: Event,
    {
        debug_assert_eq!(TypeId::of::<E>(), self.type_id);
        let mut event = MaybeUninit::new(event);

        unsafe {
            let header = self.ptr.cast::<Header>().as_ref();
            (header.call)(
                self.ptr,
                NonNull::new_unchecked(event.as_mut_ptr().cast::<()>()),
            );
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
