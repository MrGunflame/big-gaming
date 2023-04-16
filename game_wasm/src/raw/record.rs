#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct RecordReference {
    pub module: [u8; 8],
    pub record: u32,
}
