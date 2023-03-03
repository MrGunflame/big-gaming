use std::borrow::Cow;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use super::file::{FileError, FileResult};
use crate::archive::archive::ArchiveFile;
use crate::archive::module::Module;
use crate::archive::GameArchive;

/// Loader for a json file.
pub struct JsonLoader<'a> {
    archive: &'a GameArchive,
    module: &'a Module,
    root: &'a Path,
}

impl<'a> JsonLoader<'a> {
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

        let mut file = File::open(path).map_err(|err| FileError {
            path: path.to_owned(),
            msg: err.to_string(),
        })?;

        let mut buf = Vec::new();
        file.read_to_end(&mut buf).map_err(|err| FileError {
            path: path.to_owned(),
            msg: err.to_string(),
        })?;

        let archive: ArchiveFile = serde_json::from_slice(&buf).map_err(|err| FileError {
            path: path.to_owned(),
            msg: err.to_string(),
        })?;

        match archive {
            ArchiveFile::Items(items) => {
                for item in items {
                    self.archive.items().insert(item);
                }
            }
            ArchiveFile::Objects(objects) => {
                for object in objects {
                    self.archive.objects().insert(object, self.module);
                }
            }
            ArchiveFile::Components(components) => {
                unimplemented!()
            }
        }

        Ok(())
    }
}
