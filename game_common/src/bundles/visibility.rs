use bevy_ecs::bundle::Bundle;
use bevy_render::view::visibility::{ComputedVisibility, Visibility};

#[derive(Clone, Debug, Default, Bundle)]
pub struct VisibilityBundle {
    pub visibility: Visibility,
    pub computed_visibility: ComputedVisibility,
}

impl VisibilityBundle {
    pub const fn new() -> Self {
        Self {
            visibility: Visibility::Inherited,
            computed_visibility: ComputedVisibility::HIDDEN,
        }
    }
}
