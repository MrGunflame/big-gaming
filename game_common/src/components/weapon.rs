use bevy_ecs::component::Component;

/// The number of projectiles remaining in a gun.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, Component)]
pub struct Magazine(pub u16);

impl Magazine {
    /// Creates a new `Magazine` with `n` projectiles.
    #[inline]
    pub const fn new(n: u16) -> Self {
        Self(n)
    }

    #[inline]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }
}
