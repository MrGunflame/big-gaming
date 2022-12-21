use bevy::ecs::world::World;
use bevy::prelude::{Res, ResMut};
use bevy_egui::egui::Context;
use bevy_egui::egui::{Area, Order, Pos2, Ui};
use bevy_egui::EguiContext;

use crate::ui::interfaces::MENU_GAME;
use crate::ui::widgets::UiExt;
use crate::ui::{Interface, InterfaceState};

#[derive(Debug, Default)]
pub enum GameMenu {
    #[default]
    Main,
    Options,
}

impl Interface for GameMenu {
    fn create(&mut self) {}

    fn render(&mut self, ctx: &Context, world: &mut World) {
        Area::new("gamemenu")
            .fixed_pos(Pos2::new(0.0, 0.0))
            .order(Order::Foreground)
            .show(ctx, |ui| {
                ui.transparent_background(|ui| {
                    ui.vertical_centered(|ui| match self {
                        Self::Main => {
                            ui.heading("Game Menu");

                            if ui.button("Save").clicked() {}

                            if ui.button("Load").clicked() {}

                            if ui.button("Options").clicked() {
                                *self = Self::Options;
                            }

                            if ui.button("Main Menu").clicked() {}

                            if ui.button("Exit Game").clicked() {
                                std::process::exit(0);
                            }

                            ui.label("v0.1.0-aplha");
                        }
                        Self::Options => {
                            ui.label("TODO");
                        }
                    });
                });
            });
    }

    fn destroy(&mut self) {}
}
