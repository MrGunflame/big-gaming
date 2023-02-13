//! World generation
//!
//!
pub mod flat;

use super::Cell;

pub struct Generator {
    inner: Box<dyn Generate>,
}

impl Generator {
    pub fn generate(&self, cell: &mut Cell) {
        tracing::info!("Generating cell {:?}", cell.id.as_parts());

        self.inner.generate(cell);
    }
}

pub trait Generate: Send + Sync + 'static {
    /// Generates a [`Cell`] in the level for the first time.
    fn generate(&self, cell: &mut Cell);
}

impl<T> From<T> for Generator
where
    T: Generate,
{
    fn from(value: T) -> Self {
        Generator {
            inner: Box::new(value),
        }
    }
}
