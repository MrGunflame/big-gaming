use self::main_menu::MainMenuState;

pub mod main_menu;

#[derive(Debug, Default)]
pub enum GameState {
    /// Initial game startup phase.
    #[default]
    Startup,
    MainMenu(MainMenuState),
    /// Connecting to server
    Connecting,
    /// Connection failed
    ConnectionFailure,
    /// Connected to game world.
    GameWorld,
}
