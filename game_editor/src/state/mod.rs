use std::sync::mpsc;

use crate::backend::Handle;
use crate::windows::SpawnWindow;

use self::module::Modules;
use self::record::Records;

pub mod capabilities;
pub mod module;
pub mod record;

#[derive(Clone, Debug)]
pub struct EditorState {
    pub modules: Modules,
    pub records: Records,
    pub spawn_windows: mpsc::Sender<SpawnWindow>,
    pub handle: Handle,
}
