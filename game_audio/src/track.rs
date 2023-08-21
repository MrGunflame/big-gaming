use std::collections::HashMap;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum TrackId {
    Main,
    Track(slotmap::DefaultKey),
}

#[derive(Clone, Debug)]
pub struct Track {
    pub target: TrackId,
}

#[derive(Clone, Debug)]
pub(crate) struct TrackGraph {
    pub tracks: Vec<TrackId>,
}

impl TrackGraph {
    pub fn new<'a>(tracks: impl Iterator<Item = (TrackId, &'a Track)>) -> Self {
        let tracks: Vec<(TrackId, &Track)> = tracks.collect();

        let mut track_deps: HashMap<TrackId, Vec<TrackId>> = HashMap::new();
        for (id, track) in tracks {
            if track.target != TrackId::Main {
                track_deps.entry(id).or_default().push(track.target);
            }
        }

        let mut track_ids = vec![];

        while !track_deps.is_empty() {
            let mut remove_tracks = vec![];

            for (id, deps) in &track_deps {
                if deps.is_empty() {
                    remove_tracks.push(*id);
                    track_ids.push(*id);
                }
            }

            for id in remove_tracks {
                track_deps.remove(&id);
                for deps in track_deps.values_mut() {
                    deps.retain(|dep| *dep != id);
                }
            }
        }

        // Main track also at last.
        track_ids.push(TrackId::Main);

        Self { tracks: track_ids }
    }
}
