use bevy_ecs::system::Resource;
use slotmap::SlotMap;

use crate::backend::Backend;
use crate::channel::Sender;
use crate::sound::{Destination, Frame, PlayingSound, SoundId};
use crate::sound_data::{Settings, SoundData};
use crate::spatial::{Emitter, EmitterId, Listener, ListenerId};
use crate::track::{ActiveTrack, Track, TrackGraph, TrackId};

#[derive(Debug, Resource)]
pub struct AudioManager<B>
where
    B: Backend,
{
    backend: B,
    tx: Sender,
    sounds: SlotMap<slotmap::DefaultKey, PlayingSound>,
    tracks: SlotMap<slotmap::DefaultKey, ActiveTrack>,
    sample_rate: u32,
    buffer_size: u32,
    track_graph: TrackGraph,
    listeners: SlotMap<slotmap::DefaultKey, Listener>,
    emitters: SlotMap<slotmap::DefaultKey, Emitter>,
}

impl<B> AudioManager<B>
where
    B: Backend,
{
    pub fn new(mut backend: B) -> Self {
        let sample_rate = 48_000;
        let buffer_size = 3;

        let (tx, rx) = crate::channel::channel(sample_rate / 60 * buffer_size);
        backend.create_output_stream(rx);

        Self {
            backend,
            tx,
            sounds: SlotMap::new(),
            sample_rate: sample_rate as u32,
            buffer_size: buffer_size as u32,
            tracks: SlotMap::new(),
            track_graph: TrackGraph::new(std::iter::empty()),
            listeners: SlotMap::new(),
            emitters: SlotMap::new(),
        }
    }

    pub fn play(&mut self, data: SoundData, settings: Settings) -> SoundId {
        let key = self.sounds.insert(PlayingSound {
            data,
            cursor: 0,
            destination: settings.destination,
        });

        SoundId(key)
    }

    pub fn stop(&mut self, id: SoundId) {
        self.sounds.remove(id.0);
    }

    pub fn add_track(&mut self, track: Track) -> TrackId {
        let num_samples = self.sample_rate / 60 * self.buffer_size;

        let key = self.tracks.insert(ActiveTrack {
            target: track.target,
            buffer: vec![Frame::EQUILIBRIUM; num_samples as usize],
            volume: track.volume,
        });

        self.track_graph =
            TrackGraph::new(self.tracks.iter().map(|(id, t)| (TrackId::Track(id), t)));

        TrackId::Track(key)
    }

    pub fn update(&mut self) {
        let mut drop_sounds = vec![];

        let spare_cap = self.tx.spare_capacity();

        // The output buffer is still full.
        if spare_cap == 0 {
            return;
        }

        let mut main_buffer = vec![Frame::EQUILIBRIUM; spare_cap];

        // Reset all buffers from previous update.
        for track in self.tracks.values_mut() {
            track.buffer.fill(Frame::EQUILIBRIUM);
        }

        // Spatial sound
        for listener in self.listeners.values() {
            let buf = match listener.track {
                TrackId::Main => &mut main_buffer,
                TrackId::Track(key) => {
                    let track = self.tracks.get_mut(key).unwrap();
                    &mut track.buffer
                }
            };

            for (emitter_id, emitter) in self.emitters.iter() {
                for (id, sound) in self.sounds.iter_mut() {
                    let Destination::Emitter(dest_id) = sound.destination else {
                        continue;
                    };

                    if emitter_id != dest_id.0 {
                        continue;
                    }

                    for index in 0..spare_cap {
                        let Some(mut frame) = sound.data.frames.get(sound.cursor).copied() else {
                            drop_sounds.push(id);
                            break;
                        };

                        frame = crate::spatial::process(listener, emitter, frame);

                        buf[index] += frame * sound.data.volume;
                        sound.cursor += 1;
                    }
                }
            }
        }

        for &track_id in &self.track_graph.tracks {
            {
                let buf = match track_id {
                    TrackId::Main => &mut main_buffer,
                    TrackId::Track(key) => {
                        let track = self.tracks.get_mut(key).unwrap();
                        &mut track.buffer
                    }
                };

                for (id, sound) in self.sounds.iter_mut() {
                    let Destination::Track(dest_id) = sound.destination else {
                        continue;
                    };

                    if track_id != dest_id {
                        continue;
                    }

                    for index in 0..spare_cap {
                        let Some(frame) = sound.data.frames.get(sound.cursor) else {
                            drop_sounds.push(id);
                            break;
                        };

                        buf[index] += *frame * sound.data.volume;
                        sound.cursor += 1;
                    }
                }
            }

            // Done processing all sounds for this track.
            // Reroute the buffer into the target track.
            if let TrackId::Track(src_key) = track_id {
                // FIXME: Don't clone.
                let src_track = self.tracks.get(src_key).unwrap().clone();

                let target_buffer = match src_track.target {
                    TrackId::Main => &mut main_buffer,
                    TrackId::Track(dst_key) => {
                        let dst_track = self.tracks.get_mut(dst_key).unwrap();
                        &mut dst_track.buffer
                    }
                };

                for index in 0..spare_cap {
                    target_buffer[index] += src_track.buffer[index] * src_track.volume;
                }
            }
        }

        self.tx.send(&main_buffer);

        for id in drop_sounds {
            self.sounds.remove(id);
        }
    }

    pub fn add_listener(&mut self, listener: Listener) -> ListenerId {
        let key = self.listeners.insert(listener);
        ListenerId(key)
    }

    pub fn get_listener(&self, id: ListenerId) -> Option<&Listener> {
        self.listeners.get(id.0)
    }

    pub fn get_listener_mut(&mut self, id: ListenerId) -> Option<&mut Listener> {
        self.listeners.get_mut(id.0)
    }

    pub fn remove_listener(&mut self, id: ListenerId) {
        self.listeners.remove(id.0);
    }

    pub fn add_emitter(&mut self, emitter: Emitter) -> EmitterId {
        let key = self.emitters.insert(emitter);
        EmitterId(key)
    }

    pub fn get_emitter(&self, id: EmitterId) -> Option<&Emitter> {
        self.emitters.get(id.0)
    }

    pub fn get_emitter_mut(&mut self, id: EmitterId) -> Option<&mut Emitter> {
        self.emitters.get_mut(id.0)
    }

    pub fn remove_emitter(&mut self, id: EmitterId) {
        self.emitters.remove(id.0);
    }
}
