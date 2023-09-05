use std::io;
use std::path::Path;

use futures::{select, FutureExt};
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tokio::runtime::Runtime;
use tokio::sync::mpsc;

#[derive(Debug)]
pub struct AssetServer {
    rt: Runtime,
}

impl AssetServer {
    pub fn new() -> Self {
        let rt = Runtime::new().unwrap();

        Self { rt }
    }

    pub fn load<P>(&mut self, path: P) -> LoadHandle
    where
        P: AsRef<Path>,
    {
        let path = path.as_ref().to_owned();
        let (tx, rx) = mpsc::channel(32);

        self.rt.spawn(async move {
            let mut file = match File::open(path).await {
                Ok(file) => file,
                Err(err) => {
                    let _ = tx.send(Event::Error(err)).await;
                    return;
                }
            };

            loop {
                let mut buf = Vec::new();
                select! {
                    res = file.read_buf(&mut buf).fuse() => match res {
                        Ok(n) if n == 0 => break,
                        Ok(_) => (),
                        Err(err) => {
                            let _ = tx.send(Event::Error(err)).await;
                            return;
                        }
                    },
                    _ = tx.closed().fuse() => break,
                }

                if tx.send(Event::Data(buf)).await.is_err() {
                    break;
                }
            }
        });

        LoadHandle { rx }
    }
}

#[derive(Debug)]
pub enum Event {
    Data(Vec<u8>),
    Error(io::Error),
}

#[derive(Debug)]
pub struct LoadHandle {
    rx: mpsc::Receiver<Event>,
}

impl LoadHandle {
    pub fn try_recv(&mut self) -> Option<Event> {
        self.rx.try_recv().ok()
    }
}
