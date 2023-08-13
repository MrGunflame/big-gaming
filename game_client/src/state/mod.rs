use bevy_ecs::world::{FromWorld, World};
use tokio::sync::mpsc;

use bevy_ecs::system::{Commands, Res, ResMut, Resource};

use self::main_menu::MainMenuEntities;

pub mod main_menu;
mod startup;

#[derive(Resource)]
pub struct InternalGameState {
    state: GameState,
    reader: mpsc::Receiver<GameState>,
}

impl FromWorld for InternalGameState {
    fn from_world(world: &mut World) -> Self {
        let (tx, rx) = mpsc::channel(1024);

        world.insert_resource(GameStateWriter(tx));

        Self {
            reader: rx,
            state: GameState::MainMenu,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GameState {
    /// Initial game startup phase.
    Startup,
    MainMenu,
    /// Connecting to server
    Connecting,
    /// Connection failed
    ConnectionFailure,
    World,
}

#[derive(Clone, Debug, Resource)]
pub struct GameStateWriter(mpsc::Sender<GameState>);

impl GameStateWriter {
    /// Creates a new `GameStateWriter` that discards all events.
    pub fn noop() -> Self {
        let (tx, _) = mpsc::channel(1);
        Self(tx)
    }

    pub fn update(&self, state: GameState) {
        let _ = self.0.try_send(state);
    }
}

pub fn update_game_state(
    mut commands: Commands,
    mut state: ResMut<InternalGameState>,
    mut ents: ResMut<MainMenuEntities>,
) {
    while let Ok(event) = state.reader.try_recv() {
        tracing::debug!("update GameState from `{:?}` to `{:?}`", state.state, event);

        match state.state {
            GameState::MainMenu => {
                for entity in ents.0.drain(..) {
                    commands.entity(entity).despawn();
                }
            }
            _ => (),
        }

        state.state = event;
    }
}
