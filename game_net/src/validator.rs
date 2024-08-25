//! Debug validator

use std::collections::HashMap;

use game_common::net::ServerEntity;
use game_common::world::control_frame::ControlFrame;
use indexmap::IndexMap;

use crate::proto::{Frame, Header};

#[derive(Clone, Debug, Default)]
pub struct DebugValidator {
    /// Data received in a single control frame.
    cfs: IndexMap<ControlFrame, ControlFrameData>,
}

impl DebugValidator {
    pub fn new() -> Self {
        Self {
            cfs: IndexMap::new(),
        }
    }

    pub fn push(&mut self, header: Header, frame: &Frame) {
        let Some(entity) = frame.id() else {
            return;
        };

        let entry = self.cfs.entry(header.control_frame).or_default();
        let entry = entry.entities.entry(entity).or_default();

        match frame {
            Frame::EntityTranslate(frame) => match &mut entry.translation {
                Some(count) => {
                    *count += 1;
                    tracing::warn!("received {} `EntityTranslation` frames for entity {:?} in control frame {:?}", count, frame.entity, header.control_frame);
                }
                None => entry.translation = Some(1),
            },
            Frame::EntityRotate(frame) => {
                match &mut entry.rotation {
                    Some(count) => {
                        *count += 1;
                        tracing::warn!("received {} `EntityRotation` frames for entity {:?} control frame {:?}", count, frame.entity, header.control_frame);
                    }
                    None => entry.rotation = Some(1),
                }
            }
            _ => (),
        }

        if self.cfs.len() >= 8192 {
            self.cfs.pop();
        }
    }
}

#[derive(Clone, Debug, Default)]
struct ControlFrameData {
    entities: HashMap<ServerEntity, EntityData>,
}

#[derive(Clone, Debug, Default)]
struct EntityData {
    translation: Option<usize>,
    rotation: Option<usize>,
}
