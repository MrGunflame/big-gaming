#![deny(unsafe_op_in_unsafe_fn)]
#![deny(unused_crate_dependencies)]

use bevy_app::{App, Plugin};

pub mod bundles;
pub mod components;

mod actions;
mod sense;
mod thinker;

pub struct AiPlugin;

impl Plugin for AiPlugin {
    fn build(&self, app: &mut App) {
        sense::senses(app);
        thinker::thinkers(app);
        actions::actions(app);
    }
}

pub struct Sense {}
