use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};

use crate::archive::module::Module;
use crate::archive::GameArchive;

use super::file::{FileError, FileLoader, FileResult};
use super::Loader;

/// Recursively load an entire directory.
pub struct DirectoryLoader<'a> {
    archive: &'a GameArchive,
    root: &'a Module,
}

impl<'a> DirectoryLoader<'a> {
    #[inline]
    pub fn new(archive: &'a GameArchive, root: &'a Module) -> Self {
        Self { archive, root }
    }

    pub fn load<P>(&self, path: P) -> FileResult
    where
        P: AsRef<Path>,
    {
        let path = path.as_ref();

        let entries = fs::read_dir(path).map_err(|err| FileError {
            path: path.to_owned(),
            msg: err.to_string(),
        })?;

        for entry in entries {
            let entry = entry.map_err(|err| FileError {
                path: path.to_owned(),
                msg: err.to_string(),
            })?;

            let metadata = entry.metadata().map_err(|err| FileError {
                path: path.to_owned(),
                msg: err.to_string(),
            })?;

            if !metadata.is_file() {
                unimplemented!();
            }

            let path = entry.path();
            FileLoader::new(self.archive, self.root).load(path)?;
        }

        Ok(())
    }
}
