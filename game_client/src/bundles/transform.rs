use bevy::prelude::{Bundle, Transform, Vec3};
use game_common::components::transform::PreviousTransform;

#[derive(Debug, Bundle)]
pub struct TransformBundle {
    #[bundle]
    pub transform: bevy::transform::TransformBundle,
    pub previous_transform: PreviousTransform,
}

impl TransformBundle {
    pub fn new() -> Self {
        Self {
            transform: bevy::transform::TransformBundle {
                local: Transform {
                    translation: Vec3::new(0.0, 0.0, 0.0),
                    ..Default::default()
                },
                ..Default::default()
            },
            previous_transform: PreviousTransform(Transform {
                translation: Vec3::new(0.0, 0.0, 0.0),
                ..Default::default()
            }),
        }
    }

    pub fn from_translation<T>(pos: T) -> Self
    where
        T: Into<Vec3>,
    {
        let translation = pos.into();

        Self {
            transform: bevy::transform::TransformBundle {
                local: Transform {
                    translation,
                    ..Default::default()
                },
                ..Default::default()
            },
            previous_transform: PreviousTransform(Transform {
                translation,
                ..Default::default()
            }),
        }
    }
}
