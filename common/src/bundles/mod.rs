//! Entity bundles

mod actor;
mod combat;
mod physics;
mod projectile;
mod transform;
mod visibility;

pub use actor::ActorBundle;
pub use combat::CombatBundle;
pub use transform::TransformBundle;
pub use visibility::VisibilityBundle;
