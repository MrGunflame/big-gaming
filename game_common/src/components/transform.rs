use std::ops::{Deref, DerefMut};

use super::Transform;

/// The [`Transform`] of a component at the previous frame.
///
/// This can be used to calculate delta movement. It should only be added to components than can
/// move.
#[derive(Copy, Clone, Debug, Default)]
pub struct PreviousTransform(pub Transform);

impl Deref for PreviousTransform {
    type Target = Transform;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for PreviousTransform {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct GlobalTransform(pub Transform);
