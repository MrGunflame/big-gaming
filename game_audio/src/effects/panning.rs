#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Panning {
    pub left: f32,
    pub right: f32,
}

impl Default for Panning {
    fn default() -> Self {
        Self {
            left: 1.0,
            right: 1.0,
        }
    }
}
