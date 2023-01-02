use std::fs::{File, self};
use std::path::PathBuf;
use std::io;

/// Recursively load an entire directory.
pub struct DirectoryLoader {
    path: PathBuf,
}

impl DirectoryLoader {
    pub fn new<P>(path: P) -> Self
    where
        P: Into<PathBuf>
    {
        Self { path: path.into() }
    }

    pub fn load(&self) -> io::Result<()> {
        let mut entries = fs::read_dir(&self.path)?;

        for entry in entries {
            let entry = entry?;

            let metadata = entry.metadata()?;

            if !metadata.is_file() {
                unimplemented!();
            }

            let path = entry.path();
            // load(path)
        }

        Ok(())
    }
}
