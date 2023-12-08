#[derive(Clone, Debug)]
pub struct Collider {
    pub friction: f32,
    pub restitution: f32,
    pub shape: ColliderShape,
}

#[derive(Clone, Debug)]
pub enum ColliderShape {
    Cuboid(Cuboid),
}

#[derive(Copy, Clone, Debug)]
pub struct Cuboid {
    pub hx: f32,
    pub hy: f32,
    pub hz: f32,
}
