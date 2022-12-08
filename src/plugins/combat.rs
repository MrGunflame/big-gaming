mod damage;
mod health;
mod resistances;

pub use damage::{Damage, IncomingDamage};
pub use health::Health;
pub use resistances::{Resistance, Resistances};

#[derive(Copy, Clone, Debug)]
pub struct CombatPlugin;
