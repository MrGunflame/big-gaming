use std::path::Path;

use tokio::fs::File;
use tokio::io::AsyncReadExt;

use crate::{DataBuffer, Decode, Error};

#[derive(Clone, Debug)]
pub struct FileLoader {}

impl FileLoader {
    pub async fn load<P>(path: P) -> Result<DataBuffer, Error>
    where
        P: AsRef<Path>,
    {
        Self::load_inner(path.as_ref()).await
    }

    async fn load_inner(path: &Path) -> Result<DataBuffer, Error> {
        tracing::info!("loading module: {:?}", path);

        let mut file = File::open(path).await?;

        let mut buf = Vec::new();
        file.read_to_end(&mut buf).await?;

        let data = DataBuffer::decode(&buf[..])?;

        Ok(data)
    }
}
