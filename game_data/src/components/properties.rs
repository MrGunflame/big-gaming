#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Modifier {
    /// Sets the property to the given value, overwriting any existing previous formula.
    Set,
    /// Adds the given value to the previous value.
    Add,
    Mul,
}
