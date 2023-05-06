use bevy_ecs::prelude::EventReader;
use bevy_ecs::system::Query;
use game_window::events::{WindowCreated, WindowResized};
use game_window::WindowState;
use glam::Vec2;

use super::layout::LayoutTree;

pub fn layout_tree_size_window_creation(
    mut windows: Query<(&WindowState, &mut LayoutTree)>,
    mut events: EventReader<WindowCreated>,
) {
    for event in events.iter() {
        let Ok((window, mut tree)) = windows.get_mut(event.window) else {
            continue;
        };

        let size = window.0.inner_size();

        tree.resize(Vec2::new(size.width as f32, size.height as f32));
    }
}

pub fn layout_tree_window_resized(
    mut windows: Query<&mut LayoutTree>,
    mut events: EventReader<WindowResized>,
) {
    for event in events.iter() {
        let Ok(mut tree) = windows.get_mut(event.window) else {
            continue;
        };

        tree.resize(Vec2::new(event.width as f32, event.height as f32));
    }
}
