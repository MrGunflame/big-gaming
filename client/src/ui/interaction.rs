//! The quick interaction widget.

use bevy::ecs::world::EntityRef;
use bevy::prelude::With;
use bevy_egui::egui::{
    Align2, Area, Color32, Context, FontFamily, FontId, Pos2, Response, Sense, Ui, Vec2, Widget,
};
use common::components::entity::EntityName;
use common::components::interaction::Interactions;
use common::components::player::FocusedEntity;

use crate::entities::player::PlayerCharacter;

use super::{Interface, InterfaceKind, SenseExt};

#[derive(Debug, Default)]
pub struct Interaction {}

impl Interface for Interaction {
    fn kind(&self) -> InterfaceKind {
        InterfaceKind::Widget
    }

    fn create(&mut self) {}

    fn render(&mut self, ctx: &Context, world: &mut bevy::prelude::World) {
        let focused_entity = world
            .query_filtered::<&FocusedEntity, With<PlayerCharacter>>()
            .single(world);

        let FocusedEntity::Some { entity, distance } = *focused_entity else {
            return;
        };

        let Some(entity) = world.get_entity(entity) else {
            return;
        };

        Area::new("interactions")
            .fixed_pos(Pos2::new(100.0, 100.0))
            .show(ctx, |ui| {
                InteractionBox::new(entity).show(ui);
            });
    }

    fn destroy(&mut self) {}
}

struct InteractionBox<'a> {
    name: Option<&'a str>,
    interactions: &'a Interactions,
}

impl<'a> InteractionBox<'a> {
    pub fn new(entity: EntityRef<'a>) -> Self {
        let name = entity.get::<EntityName>().map(|s| s.as_str());
        let interactions = entity.get::<Interactions>().unwrap();

        Self { name, interactions }
    }

    fn show(&self, ui: &mut Ui) {
        if let Some(name) = self.name {
            ui.label(name);
        }

        for interaction in self.interactions.iter() {
            if let Some(text) = &interaction.text {
                ui.label(text);
            }
        }
    }
}

// impl<'a> Widget for InteractionBox<'a> {
//     fn ui(self, ui: &mut Ui) -> Response {
//         let size = Vec2::splat(2.0);

//         let resp = ui.allocate_response(size, Sense::none());

//         let painter = ui.painter();

//         if let Some(name) = self.name {
//             painter.text(
//                 Pos2::new(0.0, 0.0),
//                 Align2::LEFT_TOP,
//                 name,
//                 FontId {
//                     size: 5.0,
//                     family: FontFamily::default(),
//                 },
//                 Color32::RED,
//             );
//         }

//         for interaction in self.interactions.iter() {
//             painter.text(
//                 Pos2::new(0.0, 0.0),
//                 Align2::LEFT_TOP,
//                 name,
//                 FontId {
//                     size: 5.0,
//                     family: FontFamily::default(),
//                 },
//                 Color32::RED,
//             );
//         }
//         resp
//     }
// }
