use bevy::prelude::Resource;
use game_data::record::{Record, RecordBody};
use tokio::fs::{File, OpenOptions};
use tokio::io::AsyncWriteExt;
use tokio::sync::{mpsc, oneshot};

use game_data::{DataBuffer, Encode};

use crate::state::module::EditorModule;

pub struct Backend {
    rx: mpsc::Receiver<(Task, oneshot::Sender<TaskResult<()>>)>,
}

impl Backend {
    pub fn new() -> (Self, Handle) {
        let (tx, rx) = mpsc::channel(32);

        (Self { rx }, Handle { tx })
    }

    pub async fn run(mut self) {
        while let Some((task, tx)) = self.rx.recv().await {
            let res = match task {
                Task::SaveData(module) => save_data(module).await,
            };

            let _ = tx.send(res);
        }
    }
}

async fn save_data(module: EditorModule) -> TaskResult<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .open(module.path)
        .await?;

    let mut buffer = DataBuffer::new();
    buffer.header.id = module.module.id;
    buffer.items = module
        .records
        .iter()
        .map(|record| match record.body {
            RecordBody::Item(r) => r,
        })
        .collect();

    let mut buf = Vec::new();
    buffer.encode(&mut buf);

    file.write_all(&buf).await?;
    Ok(())
}

#[derive(Clone, Debug)]
pub enum Task {
    SaveData(EditorModule),
}

pub type TaskResult<T> = Result<T, std::io::Error>;

#[derive(Clone, Debug, Resource)]
pub struct Handle {
    tx: mpsc::Sender<(Task, oneshot::Sender<TaskResult<()>>)>,
}

impl Handle {
    pub fn send(&self, task: Task) {
        let (tx, rx) = oneshot::channel();
        self.tx.try_send((task, tx)).unwrap();
    }
}
