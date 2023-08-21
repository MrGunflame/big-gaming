use bevy_ecs::system::Resource;
use game_common::utils::exclusive::Exclusive;
use slotmap::SlotMap;

use crate::backend::DefaultBackend;
use crate::sound::{Frame, PlayingSound, Queue, Sender, SoundId};
use crate::sound_data::{Settings, SoundData};
use crate::track::{ActiveTrack, Track, TrackGraph, TrackId};

#[derive(Debug, Resource)]
pub struct AudioManager {
    backend: DefaultBackend,
    tx: Exclusive<Sender>,
    sounds: SlotMap<slotmap::DefaultKey, PlayingSound>,
    tracks: SlotMap<slotmap::DefaultKey, ActiveTrack>,
    sample_rate: u32,
    track_graph: TrackGraph,
}

impl AudioManager {
    pub fn new() -> Self {
        let queue = Queue::new(100_000_000);
        let (tx, rx) = queue.split();

        let backend = DefaultBackend::new(rx);

        Self {
            backend,
            tx: Exclusive::new(tx),
            sounds: SlotMap::new(),
            sample_rate: 48_000,
            tracks: SlotMap::new(),
            track_graph: TrackGraph::new(std::iter::empty()),
        }
    }

    pub fn play(&mut self, data: SoundData, settings: Settings) -> SoundId {
        let key = self.sounds.insert(PlayingSound {
            data,
            cursor: 0,
            track: settings.track,
        });

        SoundId(key)
    }

    pub fn add_track(&mut self, track: Track) -> TrackId {
        let num_samples = (self.sample_rate as f64 * (1.0 / 60.0)) * 1.05;

        let key = self.tracks.insert(ActiveTrack {
            target: track.target,
            buffer: vec![Frame::EQUILIBRIUM; num_samples as usize],
        });

        self.track_graph =
            TrackGraph::new(self.tracks.iter().map(|(id, t)| (TrackId::Track(id), t)));

        TrackId::Track(key)
    }

    pub fn update(&mut self) {
        // 1.05 to keep a small buffer.
        let num_samples = (self.sample_rate as f64 * (1.0 / 60.0)) * 1.05;

        let mut drop_sounds = vec![];

        let mut main_buffer = vec![Frame::EQUILIBRIUM; num_samples as usize];

        // Reset all buffers from previous update.
        for track in self.tracks.values_mut() {
            track.buffer.fill(Frame::EQUILIBRIUM);
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
                    if track_id != sound.track {
                        continue;
                    }

                    for index in 0..num_samples as usize {
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

                for index in 0..src_track.buffer.len() {
                    target_buffer[index] += src_track.buffer[index];
                }
            }
        }

        for elem in main_buffer {
            if elem.left.abs() > 1.0 || elem.right.abs() > 1.0 {
                tracing::warn!("clipping");
            }

            self.tx.get_mut().push(elem);
        }

        for id in drop_sounds {
            self.sounds.remove(id);
        }
    }
}
