use std::collections::VecDeque;
use std::io::Cursor;

use bevy::prelude::{Assets, AudioSource, Handle, Plugin, Res, Resource};
use parking_lot::RwLock;
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink};

pub struct AudioPlugin {
    handle: Option<OutputStreamHandle>,
}

impl AudioPlugin {
    pub fn new() -> Self {
        let (stream, handle) = OutputStream::try_default().unwrap();
        std::mem::forget(stream);

        Self {
            handle: Some(handle),
        }
    }
}

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.insert_resource(AudioServer {
            default_sink: self.handle.as_ref().map(|h| Sink::try_new(&h).unwrap()),
            queue: RwLock::default(),
        })
        .add_system(play_queued_audio);
    }
}

#[derive(Resource)]
pub struct AudioServer {
    queue: RwLock<VecDeque<Handle<AudioSource>>>,
    default_sink: Option<Sink>,
}

impl AudioServer {
    pub fn play(&self, source: Handle<AudioSource>) -> AudioHandle {
        let mut queue = self.queue.write();
        queue.push_back(source);

        AudioHandle { id: StreamId(0) }
    }
}

pub struct AudioHandle {
    id: StreamId,
}

impl AudioHandle {
    pub fn play(&self) {}

    pub fn pause(&self) {}

    pub fn toggle(&self) {}

    pub fn stop(&self) {}
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
struct StreamId(u64);

fn play_queued_audio(audio: Res<AudioServer>, assets: Res<Assets<AudioSource>>) {
    let Some(sink) = &audio.default_sink else {
        return;
    };

    let mut queue = audio.queue.write();

    let mut index = 0;
    while index < queue.len() {
        let handle = &queue[index];

        let Some(source) = assets.get(handle) else {
            index += 1;
            continue;
        };

        let reader = Cursor::new(source.clone());
        sink.append(Decoder::new(reader).unwrap());

        queue.remove(index);
    }
}
