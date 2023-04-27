//! Entity bundles

mod actor;
mod combat;
mod object;
mod player;
mod projectile;
mod transform;

pub use actor::ActorBundle;
pub use combat::CombatBundle;
pub use object::ObjectBundle;
pub use player::HostPlayerBundle;
pub use projectile::ProjectileBundle;
pub use transform::TransformBundle;
