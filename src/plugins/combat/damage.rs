use bevy::prelude::Component;

/// A raw receiving damage.
#[derive(Clone, Debug, Component)]
pub struct Damage {
    pub class: Option<()>,
    pub amount: u32,
}

impl Damage {
    pub const fn new(amount: u32) -> Self {
        Self {
            class: None,
            amount,
        }
    }
}

/// A list of incoming damage to an actor.
#[derive(Clone, Debug, Component)]
pub struct IncomingDamage {
    incoming: Vec<Damage>,
}

impl IncomingDamage {
    pub fn push(&mut self, damage: Damage) {
        self.incoming.push(damage);
    }

    pub fn pop(&mut self) -> Option<Damage> {
        self.incoming.pop()
    }
}
