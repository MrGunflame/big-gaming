use super::{CellBuilder, Generate};

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct StubGenerator;

impl Generate for StubGenerator {
    fn generate(&self, _cell: &mut CellBuilder) {}
}
