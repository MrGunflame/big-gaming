use bytemuck::{Pod, Zeroable};

#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
#[repr(transparent)]
pub struct Color(pub [f32; 4]);

impl Color {
    pub const WHITE: Self = Self([1.0, 1.0, 1.0, 1.0]);
    pub const BLACK: Self = Self([0.0, 0.0, 0.0, 1.0]);

    pub const RED: Self = Self([1.0, 0.0, 0.0, 1.0]);
    pub const GREEN: Self = Self([0.0, 1.0, 0.0, 1.0]);
    pub const BLUE: Self = Self([0.0, 0.0, 1.0, 1.0]);

    pub(crate) fn as_rgb(self) -> [f32; 3] {
        [self.0[0], self.0[1], self.0[2]]
    }

    #[inline]
    pub const fn from_rgb(rgb: [f32; 3]) -> Self {
        Self([rgb[0], rgb[1], rgb[2], 1.0])
    }

    #[inline]
    pub const fn from_rgba(rgba: [f32; 4]) -> Self {
        Self(rgba)
    }
}
