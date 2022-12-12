use bevy::prelude::Component;

use super::resistances::ResistanceId;

/// A raw receiving damage.
#[derive(Copy, Clone, Debug, Component)]
pub struct Damage {
    pub class: Option<ResistanceId>,
    pub amount: u32,
}

impl Damage {
    pub const fn new(amount: u32) -> Self {
        Self {
            class: None,
            amount,
        }
    }

    pub const fn with_class(mut self, class: ResistanceId) -> Self {
        self.class = Some(class);
        self
    }
}

/// A list of incoming damage to an actor.
#[derive(Clone, Debug, Component)]
pub struct IncomingDamage {
    incoming: Vec<Damage>,
}

impl IncomingDamage {
    pub const fn new() -> Self {
        Self {
            incoming: Vec::new(),
        }
    }

    /// Clears the list of incoming damage.
    pub fn clear(&mut self) {
        self.incoming.clear();
        self.incoming.shrink_to_fit();
    }

    pub fn push(&mut self, damage: Damage) {
        self.incoming.push(damage);
    }

    pub fn pop(&mut self) -> Option<Damage> {
        self.incoming.pop()
    }
}
