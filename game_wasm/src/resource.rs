#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ResourceId(u64);

impl ResourceId {
    #[inline]
    pub const fn to_bits(self) -> u64 {
        self.0
    }

    #[inline]
    pub const fn from_bits(bits: u64) -> Self {
        Self(bits)
    }
}
