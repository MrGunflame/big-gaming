use std::path::Path;

use self::directory::DirectoryLoader;
use self::file::FileResult;

use super::module::Module;
use super::GameArchive;

mod directory;
mod file;

#[cfg(feature = "json")]
mod json;

#[derive(Clone, Debug)]
pub struct ModuleLoader<'a> {
    archive: &'a GameArchive,
}

impl<'a> ModuleLoader<'a> {
    #[inline]
    pub fn new(archive: &'a GameArchive) -> Self {
        Self { archive }
    }

    pub fn load<P>(&self, path: P) -> FileResult
    where
        P: AsRef<Path>,
    {
        let path = path.as_ref();
        tracing::info!("Loading module {:?}", path);

        let module = Module {
            root: path.to_owned(),
        };

        DirectoryLoader::new(self.archive, &module).load(path)
    }
}

pub trait Loader {
    fn load<P>(&self, path: P) -> FileResult
    where
        P: AsRef<Path>;
}
