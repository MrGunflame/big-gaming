#![deny(unsafe_op_in_unsafe_fn)]
#![feature(const_option)]

mod effects;
pub mod sound;
pub mod track;

use std::collections::VecDeque;
use std::io::Cursor;
use std::sync::Arc;

use bevy_app::{App, Plugin};
use bevy_asset::{AddAsset, AssetLoader, Assets, Handle, LoadedAsset};
use bevy_ecs::system::{NonSendMut, Res, Resource};
use bevy_reflect::TypeUuid;

use kira::manager::backend::cpal::CpalBackend;
use kira::manager::{AudioManager, AudioManagerSettings};
use kira::sound::static_sound::{StaticSoundData, StaticSoundSettings};
use parking_lot::RwLock;

#[derive(Clone, Debug, TypeUuid)]
#[uuid = "ec630975-6b47-4006-9f16-d8b1eeb3584c"]
pub struct AudioSource {
    pub bytes: Arc<[u8]>,
}

impl AsRef<[u8]> for AudioSource {
    fn as_ref(&self) -> &[u8] {
        &self.bytes
    }
}

pub struct AudioPlugin {}

impl AudioPlugin {
    pub fn new() -> Self {
        Self {}
    }
}

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(AudioServer {
            queue: RwLock::default(),
        })
        .insert_non_send_resource(AudioBackend::new())
        .add_asset::<AudioSource>()
        .init_asset_loader::<AudioLoader>()
        .add_system(play_queued_audio);
    }
}

#[derive(Copy, Clone, Debug, Default)]
struct AudioLoader;

impl AssetLoader for AudioLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut bevy_asset::LoadContext,
    ) -> bevy_asset::BoxedFuture<'a, Result<(), bevy_asset::Error>> {
        load_context.set_default_asset(LoadedAsset::new(AudioSource {
            bytes: bytes.into(),
        }));
        Box::pin(async move { Ok(()) })
    }

    fn extensions(&self) -> &[&str] {
        &["flac", "wav", "ogg"]
    }
}

#[derive(Resource)]
pub struct AudioServer {
    queue: RwLock<VecDeque<Handle<AudioSource>>>,
}

struct AudioBackend {
    manager: Option<AudioManager<CpalBackend>>,
}

impl AudioBackend {
    fn new() -> Self {
        let manager = match AudioManager::new(AudioManagerSettings::default()) {
            Ok(man) => Some(man),
            Err(err) => {
                tracing::error!("Failed to attach to audio sink: {}", err);
                None
            }
        };

        Self { manager }
    }
}

impl Default for AudioBackend {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
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

fn play_queued_audio(
    mut backend: NonSendMut<AudioBackend>,
    audio: Res<AudioServer>,
    assets: Res<Assets<AudioSource>>,
) {
    let Some(manager) = &mut backend.manager else {
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
        let data = StaticSoundData::from_cursor(reader, StaticSoundSettings::default()).unwrap();

        let handle = manager.play(data).unwrap();

        queue.remove(index);
    }
}
