use std::path::Path;

use tokio::fs::File;
use tokio::io::AsyncReadExt;

pub async fn load_file<P>(path: P) -> Result<Vec<u8>, std::io::Error>
where
    P: AsRef<Path>,
{
    let mut file = File::open(path).await?;

    let mut buf = Vec::new();
    file.read_to_end(&mut buf).await?;

    Ok(buf)
}
