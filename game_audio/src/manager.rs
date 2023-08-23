use std::sync::Arc;
use std::time::Instant;

use bevy_ecs::system::Resource;
use parking_lot::Mutex;
use slotmap::SlotMap;

use crate::backend::DefaultBackend;
use crate::sound::{Buffer, Frame, PlayingSound, SoundId};
use crate::sound_data::{Settings, SoundData};
use crate::track::{ActiveTrack, Track, TrackGraph, TrackId};

#[derive(Debug, Resource)]
pub struct AudioManager {
    backend: DefaultBackend,
    main_buffer: Arc<Mutex<Buffer>>,
    sounds: SlotMap<slotmap::DefaultKey, PlayingSound>,
    tracks: SlotMap<slotmap::DefaultKey, ActiveTrack>,
    sample_rate: u32,
    buffer_size: u32,
    track_graph: TrackGraph,
    last_update: Instant,
}

impl AudioManager {
    pub fn new() -> Self {
        let sample_rate = 48_000;
        let buffer_size = 3;

        let mut buf = Arc::new(Mutex::new(Buffer::new(sample_rate / 60 * buffer_size)));

        let backend = DefaultBackend::new(buf.clone());

        Self {
            backend,
            sounds: SlotMap::new(),
            sample_rate: sample_rate as u32,
            buffer_size: buffer_size as u32,
            tracks: SlotMap::new(),
            track_graph: TrackGraph::new(std::iter::empty()),
            last_update: Instant::now(),
            main_buffer: buf,
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

        let spare_cap = {
            let buf = self.main_buffer.lock();
            buf.spare_capacity()
        };

        // The output buffer is still full.
        if spare_cap == 0 {
            return;
        }

        let mut main_buffer = vec![Frame::EQUILIBRIUM; spare_cap];

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

        let mut buf = self.main_buffer.lock();
        for elem in main_buffer {
            if elem.left.abs() > 1.0 || elem.right.abs() > 1.0 {
                tracing::warn!("clipping");
            }

            buf.push(elem);
        }
        drop(buf);

        for id in drop_sounds {
            self.sounds.remove(id);
        }
    }
}
