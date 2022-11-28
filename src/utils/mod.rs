#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
pub struct Degrees(pub f32);

impl Degrees {
    #[inline]
    pub const fn new(val: f32) -> Self {
        Self(val)
    }

    pub fn to_radians(self) -> Radians {
        Radians(self.0.to_radians())
    }

    pub fn to_f32(self) -> f32 {
        self.0
    }
}

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
pub struct Radians(pub f32);

impl Radians {
    pub const fn new(val: f32) -> Self {
        Self(val)
    }

    pub fn to_degrees(self) -> Degrees {
        Degrees(self.0.to_degrees())
    }

    pub fn to_f32(self) -> f32 {
        self.0
    }
}

impl From<Degrees> for Radians {
    fn from(deg: Degrees) -> Self {
        deg.to_radians()
    }
}

impl From<Radians> for Degrees {
    fn from(rad: Radians) -> Self {
        rad.to_degrees()
    }
}
