//! The template data editor.

use bevy::prelude::{Camera, Camera3dBundle, Commands, Component, Query, With};
use bevy::render::camera::RenderTarget;
use bevy::window::{Window, WindowRef};
use bevy_egui::egui::{Align, CentralPanel, Layout};
use bevy_egui::EguiContext;
use game_common::units::Mass;
use game_data::components::item::{Item, ItemId};
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TemplatesPlugin;

impl bevy::prelude::Plugin for TemplatesPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_startup_system(spawn_window)
            .add_system(render_window);
    }
}

#[derive(Copy, Clone, Debug, Component)]
struct TemplatesWindow;

fn spawn_window(mut commands: Commands) {
    let id = commands
        .spawn(Window {
            title: "Templates".to_owned(),
            ..Default::default()
        })
        .insert(TemplatesWindow)
        .id();

    commands.spawn(Camera3dBundle {
        camera: Camera {
            target: RenderTarget::Window(WindowRef::Entity(id)),
            ..Default::default()
        },
        ..Default::default()
    });
}

fn render_window(mut windows: Query<&mut EguiContext, With<TemplatesWindow>>) {
    let mut ctx = windows.single_mut();

    let items = vec![
        Item {
            id: ItemId(0),
            name: "test".to_owned(),
            mass: Mass::from_grams(10),
        },
        Item {
            id: ItemId(1),
            name: "x".to_owned(),
            mass: Mass::from_grams(11),
        },
    ];

    CentralPanel::default().show(ctx.get_mut(), |ui| {
        ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
            ui.label("ID");
            ui.label("Name");
            ui.label("Mass");
        });

        for item in items {
            ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
                ui.label(format!("{:?}", item.id));
                ui.label(item.name);
                ui.label(item.mass.to_grams().to_string());
            });
        }
    });
}
