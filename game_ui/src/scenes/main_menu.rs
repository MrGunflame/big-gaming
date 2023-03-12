use bevy::prelude::World;
use bevy_egui::egui::{Area, CentralPanel, Pos2, Ui, Window};
use game_common::scene::{Scene, SceneTransition};

use crate::{Widget, WidgetFlags};

#[derive(Debug, Default)]
pub struct MainMenu {
    state: State,
}

impl Widget for MainMenu {
    fn name(&self) -> &'static str {
        "core::main_menu"
    }

    fn flags(&self) -> crate::WidgetFlags {
        WidgetFlags::IGNORE_CLOSE
    }

    fn render(&mut self, ctx: &mut crate::Context) {
        CentralPanel::default().show(ctx.ctx, |ui| {
            render(&mut self.state, ui, ctx.world);
        });
    }
}

#[derive(Clone, Debug, Default)]
enum State {
    #[default]
    Main,
    ServerConnect {
        addr: String,
    },
}

fn render(state: &mut State, ui: &mut Ui, world: &mut World) {
    match state {
        State::Main => {
            ui.label("Main Menu");

            if ui.button("Singleplayer").clicked() {
                dbg!("single player");
            }

            if ui.button("Multiplayer").clicked() {
                *state = State::ServerConnect {
                    addr: String::new(),
                };
            }

            if ui.button("Settings").clicked() {}

            if ui.button("Exit").clicked() {
                std::process::exit(0);
            }
        }
        State::ServerConnect { addr } => {
            ui.text_edit_singleline(addr);

            if ui.button("Ok").clicked() {
                world.send_event(SceneTransition {
                    from: Scene::MainMenu,
                    to: Scene::ServerConnect { addr: addr.clone() },
                });
            }

            if ui.button("Back").clicked() {
                *state = State::Main;
            }
        }
    }
}
