mod camera;
pub mod combat;
pub mod hotkeys;
mod input;
mod movement;
mod projectile;

pub use camera::CameraPlugin;
pub use hotkeys::HotkeyPlugin;
pub use movement::MovementPlugin;
pub use projectile::ProjectilePlugin;
