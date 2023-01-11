#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use super::component::Component;
use super::items::Item;
use super::objects::Object;

/// ustar only
const MAGIC_TAR: &[u8] = &[0x75, 0x73, 0x74, 0x61, 0x72, 0x00, 0x30, 0x30];

/// `.7z` file header magic
const MAGIC_7Z: &[u8] = &[0x37, 0x7A, 0xAF, 0x27, 0x1C];

pub enum FileFormat {
    Tar,
    _7z,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "type", content = "content"))]
pub enum ArchiveFile {
    #[cfg_attr(feature = "serde", serde(rename = "item"))]
    Items(Vec<Item>),
    #[cfg_attr(feature = "serde", serde(rename = "object"))]
    Objects(Vec<Object>),
    #[cfg_attr(feature = "serde", serde(rename = "component"))]
    Components(Vec<Component>),
}
