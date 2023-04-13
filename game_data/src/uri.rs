use std::path::{Path, PathBuf};

use bytes::{Buf, BufMut};

use crate::{Decode, Encode};

/// An URI to an unspecified file.
#[derive(Clone, Debug)]
pub struct Uri {
    path: PathBuf,
}

impl Uri {
    pub fn new() -> Self {
        Self {
            path: PathBuf::new(),
        }
    }

    pub fn push_front(&mut self, mut path: PathBuf) {
        path.push(&self.path);
        self.path = path;
    }
}

impl From<PathBuf> for Uri {
    fn from(value: PathBuf) -> Self {
        Self { path: value }
    }
}

impl AsRef<Path> for Uri {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}

impl Encode for Uri {
    fn encode<B>(&self, buf: B)
    where
        B: BufMut,
    {
        // FIXME: We should encode this as a byte slice directly.
        self.path.to_str().unwrap().encode(buf);
    }
}

impl Decode for Uri {
    type Error = <String as Decode>::Error;

    fn decode<B>(buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let s = String::decode(buf)?;
        Ok(Self::from(PathBuf::from(s)))
    }
}
