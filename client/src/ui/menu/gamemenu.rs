use bevy::prelude::{Res, ResMut};
use bevy_egui::egui::{Area, Button, Color32, Order, Pos2, Ui};
use bevy_egui::EguiContext;

use crate::ui::interfaces::MENU_GAME;
use crate::ui::widgets::UiExt;
use crate::ui::{InterfaceId, InterfaceState};

pub fn gamemenu(mut egui: ResMut<EguiContext>, state: Res<InterfaceState>) {
    let state = unsafe {
        match state.get_mut::<_, State>(MENU_GAME) {
            Some(state) => state,
            None => return,
        }
    };

    Area::new("gamemenu")
        .fixed_pos(Pos2::new(0.0, 0.0))
        .order(Order::Foreground)
        .show(egui.ctx_mut(), |ui| {
            ui.transparent_background(|ui| {
                ui.vertical_centered(|ui| {
                    state.render(ui);
                });
            });
        });
}

/// State of the Gamemenu interface.
#[derive(Clone, Debug, Default)]
pub enum State {
    #[default]
    Main,
    Options,
}

impl State {
    fn render(&mut self, ui: &mut Ui) {
        match self {
            Self::Main => render_main(self, ui),
            Self::Options => render_options(ui),
        }
    }
}

fn render_main(state: &mut State, ui: &mut Ui) {
    ui.heading("Game Menu");

    if ui.button("Save").clicked() {}

    if ui.button("Load").clicked() {}

    if ui.button("Options").clicked() {
        *state = State::Options;
    }

    if ui.button("Main Menu").clicked() {}

    if ui.button("Exit Game").clicked() {
        std::process::exit(0);
    }

    ui.label("v0.1.0-aplha");
}

fn render_options(ui: &mut Ui) {
    ui.label("TODO");
}
