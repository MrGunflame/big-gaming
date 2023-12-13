use glam::{Quat, Vec3};

#[derive(Clone, Debug, Default)]
pub struct ActorProperties {
    /// The rotation that the actor is facing in.
    ///
    /// Defaults to [`IDENTITY`].
    ///
    /// [`IDENTITY`]: Quat::IDENTITY
    pub rotation: Quat,
    /// The local offset (from the actor root) at which the camera sits.
    pub eyes: Vec3,
    // TODO: Add custom props
}
