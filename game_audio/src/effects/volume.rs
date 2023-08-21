#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
pub struct Volume(pub f32);

impl Default for Volume {
    #[inline]
    fn default() -> Self {
        Self(1.0)
    }
}
