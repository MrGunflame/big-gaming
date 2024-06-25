use game_ui::reactive::Context;
use game_ui::widgets::{Callback, Widget};

use crate::backend::{Handle, Task};
use crate::widgets::explorer::{Entry, Explorer};

pub struct OpenModule {
    pub handle: Handle,
}

impl Widget for OpenModule {
    fn mount<T>(self, parent: &Context<T>) -> Context<()> {
        Explorer {
            on_open: on_open(self.handle),
        }
        .mount(parent)
    }
}

fn on_open(handle: Handle) -> Callback<Vec<Entry>> {
    Callback::from(move |entries: Vec<Entry>| {
        for entry in entries {
            handle.send(Task::ReadModule(entry.path));
        }
    })
}
