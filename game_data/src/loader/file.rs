use std::path::Path;

use tokio::fs::File;
use tokio::io::AsyncReadExt;

use crate::{DataBuffer, Decode};

#[derive(Clone, Debug)]
pub struct FileLoader {}

impl FileLoader {
    pub async fn load<P>(path: P) -> std::io::Result<DataBuffer>
    where
        P: AsRef<Path>,
    {
        Self::load_inner(path.as_ref()).await
    }

    async fn load_inner(path: &Path) -> std::io::Result<DataBuffer> {
        tracing::info!("loading module: {:?}", path);

        let mut file = File::open(path).await?;

        let mut buf = Vec::new();
        file.read_to_end(&mut buf).await?;

        let data = DataBuffer::decode(&buf[..])?;

        Ok(data)
    }
}
