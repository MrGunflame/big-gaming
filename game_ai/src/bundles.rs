use bevy::prelude::Bundle;

use crate::components::{Accuracy, AiController, Perception};
use crate::sense::Vision;

#[derive(Bundle, Default)]
pub struct AiBundle {
    ai_controller: AiController,
    vision: Vision,

    pub perception: Perception,
    pub accuracy: Accuracy,
}
