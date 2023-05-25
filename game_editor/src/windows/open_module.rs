use std::path::PathBuf;

use bevy_ecs::prelude::Entity;
use game_ui::reactive::Scope;
use game_ui::{component, view};

use crate::backend::{Handle, Task};
use crate::widgets::explorer::*;

use super::{SpawnWindow, SpawnWindowQueue};

#[component]
pub fn OpenModule(cx: &Scope, window: Entity, handle: Handle, queue: SpawnWindowQueue) -> Scope {
    view! {
        cx,
        <Explorer path={PathBuf::from("./")} on_cancel={on_cancel(queue.clone(), window)} on_open={on_open(handle, queue, window)}>
        </Explorer>
    }
}

fn on_cancel(queue: SpawnWindowQueue, id: Entity) -> Box<dyn Fn() + Send + Sync + 'static> {
    Box::new(move || {
        let mut queue = queue.0.write();
        queue.push_back(SpawnWindow::CloseWindow(id));
    })
}

fn on_open(
    handle: Handle,
    queue: SpawnWindowQueue,
    id: Entity,
) -> Box<dyn Fn(Vec<Entry>) + Send + Sync + 'static> {
    Box::new(move |entries| {
        for entry in entries {
            handle.send(Task::ReadModule(entry.path));
        }

        let mut queue = queue.0.write();
        queue.push_back(SpawnWindow::CloseWindow(id));
    })
}
