use bevy::prelude::Bundle;

use crate::components::AiController;
use crate::sense::Vision;

#[derive(Bundle, Default)]
pub struct AiBundle {
    ai_controller: AiController,
    vision: Vision,
}
