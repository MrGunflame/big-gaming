use bevy::prelude::Plugin;

pub mod bundles;
pub mod components;

mod actions;
mod sense;
mod thinker;

pub struct AiPlugin;

impl Plugin for AiPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        sense::senses(app);
        thinker::thinkers(app);
        actions::actions(app);
    }
}

pub struct Sense {}
