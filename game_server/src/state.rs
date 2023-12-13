use std::ops::Deref;
use std::sync::Arc;

use game_common::world::control_frame::ControlFrame;
use parking_lot::Mutex;

use crate::config::Config;
use crate::conn::Connections;

#[derive(Clone, Debug)]
pub struct State(Arc<StateInner>);

impl State {
    pub fn new(config: Config) -> Self {
        State(Arc::new(StateInner {
            config,
            conns: Connections::default(),
            control_frame: Mutex::default(),
        }))
    }
}

impl Deref for State {
    type Target = StateInner;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug)]
pub struct StateInner {
    pub config: Config,
    pub conns: Connections,
    // TODO: This can probably be AtomicU32, but needs to be consitent.
    pub control_frame: Mutex<ControlFrame>,
}
