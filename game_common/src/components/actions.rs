use std::fmt::{self, Debug, Formatter};
use std::sync::Arc;

use crate::entity::EntityId;
use crate::world::world::WorldViewMut;

use super::items::ItemId;

#[derive(Clone, Debug, Default)]
pub struct Actions {
    actions: Vec<Action>,
}

impl Actions {
    pub fn new() -> Self {
        Self {
            actions: Vec::new(),
        }
    }

    pub fn push(&mut self, action: Action) {
        self.actions.push(action);
    }
}

#[derive(Clone)]
pub struct Action {
    pub name: String,
    pub fire: Arc<dyn FireAction>,
}

impl Debug for Action {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Action")
            .field("name", &self.name)
            .field("fire", &Arc::as_ptr(&self.fire))
            .finish()
    }
}

#[derive(Debug)]
pub struct Context<'a> {
    pub id: ItemId,
    pub invoker: EntityId,
    pub world: WorldViewMut<'a>,
}

pub trait FireAction: 'static + Send + Sync {
    fn call(&self, ctx: Context<'_>);
}
