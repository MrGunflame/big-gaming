use crate::world::gen::Generate;
use crate::world::Cell;

pub struct FlatGenerator;

impl Generate for FlatGenerator {
    fn generate(&self, cell: &mut Cell) {
        dbg!(cell.id);
    }
}
