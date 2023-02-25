//! Entity bundles

mod actor;
mod combat;
mod object;
mod physics;
mod player;
mod projectile;
mod transform;
mod visibility;

pub use actor::ActorBundle;
pub use combat::CombatBundle;
pub use object::ObjectBundle;
pub use player::HostPlayerBundle;
pub use projectile::ProjectileBundle;
pub use transform::TransformBundle;
pub use visibility::VisibilityBundle;
