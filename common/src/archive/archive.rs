/// ustar only
const MAGIC_TAR: &[u8] = &[0x75, 0x73, 0x74, 0x61, 0x72, 0x00, 0x30, 0x30];

/// `.7z` file header magic
const MAGIC_7Z: &[u8] = &[0x37, 0x7A, 0xAF, 0x27, 0x1C];

pub enum FileFormat {
    Tar,
    _7z,
}

impl FileFormat {}

#[derive(Clone, Debug)]
pub struct FileHeader {
    /// The type of data contained in this file.
    kind: FileKind,
}

/// The type of items presented in a file.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct FileKind(u16);

impl FileKind {
    /// The file contains a list of [`Item`]s.
    ///
    /// [`Item`]: super::items::Item
    pub const ITEMS: Self = Self(1);
}
