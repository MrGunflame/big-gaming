#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(C)]
pub struct Item {
    pub name: *const u8,
}

pub unsafe fn define_item(item: *const Item) {}
