use std::any::Any;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Properties {
    props: HashMap<PropertyId, u32>,
}

#[derive(Copy, Clone, Debug)]
pub struct PropertyId(u32);

impl PropertyId {
    pub const DAMAGE: Self = Self(1);

    pub const ATTACK_SPEED: Self = Self(2);

    pub const MAGAZINE: Self = Self(1);
}
