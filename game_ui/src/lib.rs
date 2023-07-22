//! UI related systems

#![deny(unsafe_op_in_unsafe_fn)]
#![deny(unused_crate_dependencies)]

// We need criterion for benches, but it is incorrectly detected
// by `unused_crate_dependencies`.
#[cfg(test)]
use criterion as _;

pub mod events;
pub mod render;
pub mod widgets;

pub mod reactive;

use bevy_app::{App, Plugin};
use bevy_ecs::schedule::IntoSystemConfig;
use bevy_ecs::system::{Commands, Query, Res};
use bevy_ecs::world::World;
use events::{Events, WindowEvent, WindowEventQueue};
use game_window::WindowState;
use reactive::{Document, Runtime};
use render::layout::LayoutTree;

pub use game_ui_macros::{component, view};

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(render::RenderUiPlugin);

        app.insert_resource(Runtime::new());
        app.insert_resource(WindowEventQueue::default());

        // Events
        app.add_system(events::update_events_from_layout_tree);
        app.add_system(events::dispatch_cursor_moved_events);
        app.add_system(events::dispatch_mouse_button_input_events);
        app.add_system(events::dispatch_received_character_events);
        app.add_system(events::dispatch_keyboard_input_events);
        app.add_system(events::dispatch_mouse_wheel_events);

        app.add_system(run_effects);
        app.add_system(flush_node_queue.after(run_effects));
        app.add_system(run_window_events.after(run_effects));
    }
}

fn run_effects(world: &World, windows: Query<&Document>) {
    for doc in &windows {
        doc.run_effects(world)
    }
}

fn flush_node_queue(mut windows: Query<(&Document, &mut LayoutTree, &mut Events)>) {
    for (doc, mut tree, mut events) in &mut windows {
        doc.flush_node_queue(&mut tree, &mut events);
    }
}

fn run_window_events(
    mut commands: Commands,
    queue: Res<WindowEventQueue>,
    windows: Query<&WindowState>,
) {
    let mut queue = queue.inner.lock();
    while let Some(event) = queue.pop_front() {
        let id = match event {
            WindowEvent::Close(id) => id,
            WindowEvent::SetTitle(id, _) => id,
            WindowEvent::SetCursorIcon(id, _) => id,
        };

        if let Ok(window) = windows.get(id) {
            match event {
                WindowEvent::Close(_) => {
                    commands.entity(id).despawn();
                }
                WindowEvent::SetTitle(_, title) => {
                    window.set_title(&title);
                }
                WindowEvent::SetCursorIcon(_, icon) => {
                    window.set_cursor_icon(icon);
                }
            }
        }
    }
}
