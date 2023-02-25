use bevy_egui::egui::{Area, Pos2, Ui};

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
        Area::new("main_menu")
            .fixed_pos(Pos2::new(0.0, 0.0))
            .show(ctx.ctx, |ui| {
                render(&mut self.state, ui);
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

fn render(state: &mut State, ui: &mut Ui) {
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
        }
        State::ServerConnect { addr } => {
            ui.text_edit_singleline(addr);

            if ui.button("Back").clicked() {
                *state = State::Main;
            }
        }
    }
}
