use bevy::prelude::{Bundle, ComputedVisibility, Visibility};

#[derive(Bundle)]
pub struct VisibilityBundle {
    visibility: Visibility,
    computed_visibility: ComputedVisibility,
}

impl VisibilityBundle {
    pub const fn new() -> Self {
        Self {
            visibility: Visibility::VISIBLE,
            computed_visibility: ComputedVisibility::INVISIBLE,
        }
    }
}
