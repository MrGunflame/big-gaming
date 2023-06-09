use std::path::PathBuf;

use bevy_ecs::prelude::Entity;
use game_ui::reactive::Scope;
use game_ui::{component, view};

use crate::backend::{Handle, Task};
use crate::widgets::explorer::*;

use super::{SpawnWindow, SpawnWindowQueue};

#[component]
pub fn OpenModule(cx: &Scope, window: Entity, handle: Handle) -> Scope {
    view! {
        cx,
        <Explorer on_open={on_open(handle, window)}>
        </Explorer>
    }
}

fn on_open(handle: Handle, id: Entity) -> Box<dyn Fn(Vec<Entry>) + Send + Sync + 'static> {
    Box::new(move |entries| {
        for entry in entries {
            handle.send(Task::ReadModule(entry.path));
        }
    })
}
