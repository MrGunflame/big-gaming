use std::borrow::Cow;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::archive::module::Module;
use crate::archive::GameArchive;

use super::json::JsonLoader;
use super::Loader;

pub type FileResult = Result<(), FileError>;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum FileKind {
    #[cfg(feature = "json")]
    Json,
}

#[derive(Clone, Debug)]
pub struct FileError {
    pub path: PathBuf,
    pub msg: String,
}

pub struct FileLoader<'a> {
    archive: &'a GameArchive,
    root: &'a Module,
}

impl<'a> FileLoader<'a> {
    #[inline]
    pub fn new(archive: &'a GameArchive, root: &'a Module) -> Self {
        Self { archive, root }
    }
}

impl<'a> Loader for FileLoader<'a> {
    fn load<P>(&self, path: P) -> FileResult
    where
        P: AsRef<Path>,
    {
        let path = path.as_ref();

        match path.extension() {
            Some(ext) if ext == OsStr::new("json") => {
                JsonLoader::new(self.archive, self.root).load(path)
            }
            _ => {
                return Err(FileError {
                    path: path.to_owned(),
                    msg: "unknown or missing file type".into(),
                })
            }
        }
    }
}
