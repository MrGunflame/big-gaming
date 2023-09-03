pub mod main_menu;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum GameState {
    /// Initial game startup phase.
    #[default]
    Startup,
    MainMenu,
    /// Connecting to server
    Connecting,
    /// Connection failed
    ConnectionFailure,
    World,
}
