use bevy::prelude::{Bundle, Transform, Vec3};

use crate::prev_transform::PreviousTransform;

#[derive(Debug, Bundle)]
pub struct TransformBundle {
    // pub transform: bevy::transform::TransformBundle,
    pub previous_transform: PreviousTransform,
}

impl TransformBundle {
    pub fn from_translation<T>(pos: T) -> Self
    where
        T: Into<Vec3>,
    {
        let translation = pos.into();

        Self {
            // transform: bevy::transform::TransformBundle {
            //     local: Transform {
            //         translation,
            //         ..Default::default()
            //     },
            //     ..Default::default()
            // },
            previous_transform: PreviousTransform(Transform {
                translation,
                ..Default::default()
            }),
        }
    }
}
