use std::num::NonZeroU8;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Track(NonZeroU8);

impl Track {
    pub const HOST: Self = Self(NonZeroU8::new(1).unwrap());

    pub const LEVEL: Self = Self(NonZeroU8::new(2).unwrap());
}
