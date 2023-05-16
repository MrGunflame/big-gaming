use std::collections::VecDeque;
use std::path::PathBuf;

use bevy_ecs::system::Resource;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::oneshot::error::TryRecvError;
use tokio::sync::{mpsc, oneshot};

use game_data::{DataBuffer, Decode, Encode};

use crate::state::capabilities::Capabilities;
use crate::state::module::EditorModule;
use crate::state::record::Records;

pub struct Backend {
    rx: mpsc::Receiver<(Task, oneshot::Sender<Response>)>,
}

impl Backend {
    pub fn new() -> (Self, Handle) {
        let (tx, rx) = mpsc::channel(32);

        (
            Self { rx },
            Handle {
                tx,
                recvs: VecDeque::new(),
            },
        )
    }

    pub async fn run(mut self) {
        while let Some((task, tx)) = self.rx.recv().await {
            let resp = match task {
                Task::ReadModule(path) => Response::LoadModule(load_module(path).await),
                Task::WriteModule(module) => Response::WriteModule(save_data(module).await),
            };

            let _ = tx.send(resp);
        }
    }
}

async fn load_module(path: PathBuf) -> TaskResult<(EditorModule, Records)> {
    let mut file = File::open(&path).await?;

    let mut buf = Vec::new();
    file.read_to_end(&mut buf).await?;

    let data = DataBuffer::decode(&buf[..])?;

    let mut records = Records::new();
    for record in data.records {
        records.insert(data.header.module.id, record);
    }

    Ok((
        EditorModule {
            module: data.header.module,
            path,
            capabilities: Capabilities::NONE,
        },
        records,
    ))
}

async fn save_data(payload: WriteModule) -> TaskResult<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .open(payload.module.path)
        .await?;

    let id = payload.module.module.id;

    let mut buffer = DataBuffer::new(payload.module.module);
    // buffer.items = module
    //     .records
    //     .iter()
    //     .map(|record| match record.body {
    //         RecordBody::Item(r) => r,
    //     })
    //     .collect();

    buffer.records = payload
        .records
        .iter()
        .filter(|(m, _)| *m == id)
        .map(|(_, record)| record.clone())
        .collect();

    buffer.header.records = buffer.records.len() as u32;

    let mut buf = Vec::new();
    buffer.encode(&mut buf);

    file.write_all(&buf).await?;
    Ok(())
}

#[derive(Clone, Debug)]
pub enum Task {
    ReadModule(PathBuf),
    WriteModule(WriteModule),
}

#[derive(Clone, Debug)]
pub struct WriteModule {
    pub module: EditorModule,
    pub records: Records,
}

pub type TaskResult<T> = Result<T, game_data::Error>;

#[derive(Debug, Resource)]
pub struct Handle {
    tx: mpsc::Sender<(Task, oneshot::Sender<Response>)>,
    recvs: VecDeque<oneshot::Receiver<Response>>,
}

impl Handle {
    pub fn send(&mut self, task: Task) {
        let (tx, rx) = oneshot::channel();
        self.tx.try_send((task, tx)).unwrap();
        self.recvs.push_back(rx);
    }

    pub fn recv(&mut self) -> Option<Response> {
        let mut index = 0;
        while index < self.recvs.len() {
            let rx = &mut self.recvs[index];

            match rx.try_recv() {
                Ok(resp) => {
                    self.recvs.remove(index);
                    return Some(resp);
                }
                Err(TryRecvError::Empty) => (),
                Err(TryRecvError::Closed) => panic!("channel closed"),
            }

            index += 1;
        }

        None
    }
}

#[derive(Debug)]
pub enum Response {
    LoadModule(TaskResult<(EditorModule, Records)>),
    WriteModule(TaskResult<()>),
}
