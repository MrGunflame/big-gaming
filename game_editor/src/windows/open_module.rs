use game_ui::reactive::Scope;
use game_ui::widgets::{Callback, Widget};

use crate::backend::{Handle, Task};
use crate::widgets::explorer::{Entry, Explorer};

pub struct OpenModule {
    pub handle: Handle,
}

impl Widget for OpenModule {
    fn build(self, cx: &Scope) -> Scope {
        cx.append(Explorer {
            on_open: on_open(self.handle),
        })
    }
}

fn on_open(handle: Handle) -> Callback<Vec<Entry>> {
    Callback::from(move |entries: Vec<Entry>| {
        for entry in entries {
            handle.send(Task::ReadModule(entry.path));
        }
    })
}
