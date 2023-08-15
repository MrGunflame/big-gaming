use game_ui::reactive::Scope;
use game_ui::{component, view};

use crate::backend::{Handle, Task};
use crate::widgets::explorer::*;

#[component]
pub fn OpenModule(cx: &Scope, handle: Handle) -> Scope {
    view! {
        cx,
        <Explorer on_open={on_open(handle)}>
        </Explorer>
    }
}

fn on_open(handle: Handle) -> Box<dyn Fn(Vec<Entry>) + Send + Sync + 'static> {
    Box::new(move |entries| {
        for entry in entries {
            handle.send(Task::ReadModule(entry.path));
        }
    })
}
