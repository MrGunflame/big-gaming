use std::ops::Deref;
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::Arc;

use game_common::world::control_frame::ControlFrame;

use crate::config::Config;
use crate::conn::Connections;

#[derive(Clone, Debug)]
pub struct State(Arc<StateInner>);

impl State {
    pub fn new(config: Config) -> Self {
        State(Arc::new(StateInner {
            config,
            conns: Connections::default(),
            control_frame: AtomicControlFrame::new(),
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
    pub control_frame: AtomicControlFrame,
}

/// An atomic cell for a [`ControlFrame`].
#[derive(Debug)]
#[repr(transparent)]
pub struct AtomicControlFrame {
    // FIXME: What ordering is really required here?
    inner: AtomicU16,
}

impl AtomicControlFrame {
    #[inline]
    pub const fn new() -> Self {
        Self {
            inner: AtomicU16::new(0),
        }
    }

    #[inline]
    pub fn set(&self, frame: ControlFrame) {
        // Write using a release ordering, which is synchronized
        // with the acquire ordering loading the value.
        self.inner.store(frame.0, Ordering::Release);
    }

    #[inline]
    pub fn get(&self) -> ControlFrame {
        // By loading with an acquire load we ensure that
        // we load the most recently written control frame.
        ControlFrame(self.inner.load(Ordering::Acquire))
    }

    #[inline]
    pub fn inc(&self) {
        self.inner.fetch_add(1, Ordering::Release);
    }
}

impl Default for AtomicControlFrame {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}
