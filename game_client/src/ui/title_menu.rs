use std::sync::{mpsc, Arc};

use game_ui::reactive::Context;
use game_ui::widgets::{Button, Callback, Container, Input, Text, Widget};
use parking_lot::Mutex;

#[derive(Clone, Debug)]
pub struct TitleMenu {
    pub events: mpsc::Sender<MenuEvent>,
}

impl Widget for TitleMenu {
    fn mount<T>(self, parent: &Context<T>) -> Context<()> {
        let root = Container::new().mount(parent);

        for option in [
            MenuOption {
                name: "Continue".to_owned(),
                on_click: Callback::default(),
            },
            MenuOption {
                name: "New Game".to_owned(),
                on_click: Callback::default(),
            },
            MenuOption {
                name: "Load Game".to_owned(),
                on_click: Callback::default(),
            },
            MenuOption {
                name: "Multiplayer".to_owned(),
                on_click: {
                    let events = self.events.clone();
                    Callback::from(move |()| {
                        events.send(MenuEvent::SpawnMultiPlayerMenu).unwrap();
                    })
                },
            },
            MenuOption {
                name: "Options".to_owned(),
                on_click: Callback::default(),
            },
            MenuOption {
                name: "Exit".to_owned(),
                on_click: {
                    let events = self.events.clone();
                    Callback::from(move |()| {
                        events.send(MenuEvent::Exit).unwrap();
                    })
                },
            },
        ] {
            let button = Button::new().on_click(option.on_click).mount(&root);
            Text::new(option.name).mount(&button);
        }

        root
    }
}

#[derive(Clone, Debug)]
struct MenuOption {
    name: String,
    on_click: Callback<()>,
}

#[derive(Clone, Debug)]
pub struct MultiPlayerMenu {
    pub events: mpsc::Sender<MenuEvent>,
}

impl Widget for MultiPlayerMenu {
    fn mount<T>(self, parent: &Context<T>) -> Context<()> {
        let root = Container::new().mount(parent);

        let value = Arc::new(Mutex::new(String::new()));

        Input::new()
            .on_change({
                let value = value.clone();
                Callback::from(move |val| {
                    *value.lock() = val;
                })
            })
            .mount(&root);

        {
            let button = Button::new()
                .on_click({
                    let events = self.events.clone();
                    move |()| {
                        events.send(MenuEvent::SpawnMainMenu).unwrap();
                    }
                })
                .mount(&root);
            Text::new("Back").mount(&button);
        }

        {
            let button = Button::new()
                .on_click(move |()| {
                    self.events
                        .send(MenuEvent::Connect(value.lock().clone()))
                        .unwrap();
                })
                .mount(&root);
            Text::new("Connect").mount(&button);
        }

        root
    }
}

#[derive(Clone, Debug)]
pub enum MenuEvent {
    SpawnMainMenu,
    SpawnMultiPlayerMenu,
    Connect(String),
    Exit,
}
