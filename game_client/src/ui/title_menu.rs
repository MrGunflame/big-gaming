use std::sync::{mpsc, Arc};

use game_ui::runtime::Context;
use game_ui::widgets::{Button, Callback, Container, Input, Text, Widget};
use parking_lot::Mutex;

pub struct TitleMenu {
    pub events: mpsc::Sender<MenuEvent>,
}

impl Widget for TitleMenu {
    fn mount(self, parent: &Context) -> Context {
        let root = Container::new().mount(parent);

        let ctx = root.clone();
        let on_state_change = on_state_change(self.events, ctx);

        on_state_change.call(State::Main);
        root
    }
}

fn on_state_change(events: mpsc::Sender<MenuEvent>, ctx: Context) -> Callback<State> {
    Callback::from(move |state| {
        let events = events.clone();
        ctx.clear_children();

        match state {
            State::Main => MainTitleMenu {
                state_change: on_state_change(events.clone(), ctx.clone()),
                events,
            }
            .mount(&ctx),
            State::Connect => MultiPlayerMenu {
                on_state_change: on_state_change(events.clone(), ctx.clone()),
                events,
            }
            .mount(&ctx),
        };
    })
}

#[derive(Copy, Clone, Debug)]
enum State {
    Main,
    Connect,
}

#[derive(Clone, Debug)]
struct MainTitleMenu {
    events: mpsc::Sender<MenuEvent>,
    state_change: Callback<State>,
}

impl Widget for MainTitleMenu {
    fn mount(self, parent: &Context) -> Context {
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
                    Callback::from(move |()| {
                        self.state_change.call(State::Connect);
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
struct MultiPlayerMenu {
    events: mpsc::Sender<MenuEvent>,
    on_state_change: Callback<State>,
}

impl Widget for MultiPlayerMenu {
    fn mount(self, parent: &Context) -> Context {
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
                    move |()| {
                        self.on_state_change.call(State::Main);
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
    Connect(String),
    Exit,
}
