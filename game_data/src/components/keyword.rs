pub struct Keyword {}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Operation {
    /// Add a keyword to a template.
    Add,
    /// Removes a keyworld from an template.
    Remove,
}
