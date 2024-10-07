use std::any::TypeId;
use std::collections::HashMap;
use std::sync::Arc;

pub trait Event: Sized + 'static {}

pub struct EventHandlers {
    map: HashMap<TypeId, EventHandler>,
}

#[derive(Clone)]
struct EventHandler(Arc<dyn Fn() + 'static>);

#[derive(Debug)]
struct EventHandlerHandle();
