use std::sync::Arc;

use bevy_egui::egui::{CentralPanel, Pos2, Window};
use game_common::scene::{Scene, SceneTransition};

use crate::{Context, Widget, WidgetFlags};

#[derive(Debug)]
pub struct ServerError {
    error: Arc<dyn std::error::Error + Send + Sync + 'static>,
}

impl ServerError {
    pub fn new(error: Arc<dyn std::error::Error + Send + Sync + 'static>) -> Self {
        Self { error }
    }
}

impl Widget for ServerError {
    fn name(&self) -> &'static str {
        "core::server_error"
    }

    fn flags(&self) -> crate::WidgetFlags {
        WidgetFlags::IGNORE_CLOSE
    }

    fn render(&mut self, ctx: &mut Context) {
        CentralPanel::default().show(ctx.ctx, |ui| {
            ui.label("Server Error:");
            ui.label(self.error.to_string());

            if ui.button("Main Menu").clicked() {
                ctx.world.send_event(SceneTransition {
                    to: Scene::MainMenu,
                    from: Scene::Loading,
                });
            }
        });
    }
}
