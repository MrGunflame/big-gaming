use std::ffi::OsStr;
use std::fs::{self, File};
use std::path::{Path, PathBuf};

use crate::archive::module::Module;
use crate::archive::GameArchive;

use super::file::{FileError, FileLoader, FileResult};
use super::Loader;

/// Recursively load an entire directory.
pub struct DirectoryLoader<'a> {
    archive: &'a GameArchive,
    module: &'a Module,
    root: &'a Path,
}

impl<'a> DirectoryLoader<'a> {
    #[inline]
    pub fn new(archive: &'a GameArchive, module: &'a Module, root: &'a Path) -> Self {
        Self {
            archive,
            module,
            root,
        }
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

            let path = entry.path();

            if metadata.is_dir() {
                DirectoryLoader::new(self.archive, self.module, self.root).load(path)?;
            } else {
                if path.file_name() == Some(OsStr::new("mod.json")) {
                    continue;
                }

                FileLoader::new(self.archive, self.module, self.root).load(path)?;
            }
        }

        Ok(())
    }
}
