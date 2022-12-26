use bevy::prelude::With;
use bevy_egui::egui::{
    Align, Area, Color32, Layout, Order, Pos2, Rect, Response, Rounding, Sense, Stroke, Ui, Vec2,
    Widget,
};
use common::archive::GameArchive;
use common::components::inventory::Inventory;
use common::components::items::{ItemId, ItemStack};

use crate::entities::player::PlayerCharacter;
use crate::ui::widgets::UiExt;
use crate::ui::Interface;

const CATEGORIES: &[&'static str] = &["Weapons", "Armor", "Utility", "Scrap", "Ammo"];

#[derive(Debug, Default)]
pub struct InventoryMenu {}

impl Interface for InventoryMenu {
    fn create(&mut self) {}

    fn render(&mut self, ctx: &bevy_egui::egui::Context, world: &mut bevy::prelude::World) {
        let inventory = world
            .query_filtered::<&Inventory, With<PlayerCharacter>>()
            .single(world);

        let archive = world.resource::<GameArchive>();

        Area::new("inventory")
            .fixed_pos(Pos2::new(0.0, 0.0))
            .order(Order::Foreground)
            .show(ctx, |ui| {
                ui.transparent_background(|ui| {
                    let total_width = ui.available_width();

                    let inventory_width = total_width * (7.0 / 12.0);
                    let equip_width = total_width * (5.0 / 12.0);

                    // ui.heading("Inventory");

                    // ui.label("Filter");
                    // ui.label("Sort");
                    // ui.label("Search");

                    // ui.label(format!("{} items", inventory.items()));

                    let mut inventory_ui = ui.child_ui(
                        Rect {
                            min: Pos2::new(0.0, 0.0),
                            max: Pos2::new(inventory_width, 0.0),
                        },
                        Layout::top_down(Align::TOP),
                    );

                    for title in CATEGORIES {
                        let mut iter = inventory.iter();

                        for stack in inventory {
                            inventory_ui.add(Category {
                                archive,
                                title,
                                items: inventory.iter(),
                            });
                        }
                    }

                    // for stack in inventory {
                    //     let item = archive.item(stack.item.id).unwrap();
                    //     ui.add(Tile::square(32.0));
                    //     ui.label(format!("{:?} x{}", item.name, stack.quantity));
                    // }
                });
            });
    }

    fn destroy(&mut self) {}
}

#[derive(Debug)]
struct Category<'a, I>
where
    I: Iterator<Item = &'a ItemStack>,
{
    archive: &'a GameArchive,
    title: &'a str,
    items: I,
}

impl<'a, I> Widget for Category<'a, I>
where
    I: Iterator<Item = &'a ItemStack>,
{
    fn ui(self, ui: &mut Ui) -> Response {
        // let rect = Rect {
        //     min: Pos2::new(0.0, 0.0),
        //     max: Pos2::new(100.0, 200.0),
        // };

        // let resp = ui.allocate_rect(
        //     rect,
        //     Sense {
        //         click: false,
        //         drag: false,
        //         focusable: false,
        //     },
        // );

        // if ui.is_rect_visible(resp.rect) {
        ui.label(self.title);

        // let mut ui = ui.child_ui(
        //     Rect {
        //         min: Pos2::new(32.0, 32.0),
        //         max: Pos2::new(1000.0, 1000.0),
        //     },
        //     Layout::left_to_right(Align::TOP),
        // );

        let resp = ui.horizontal_wrapped(|ui| {
            for id in self.items {
                ui.add(Tile::new(self.archive, id).square(64.0));
            }
        });
        // }

        // resp
        resp.response
    }
}

/// A tile/cell for a single item.
struct Tile<'a> {
    archive: &'a GameArchive,
    item: &'a ItemStack,
    width: f32,
    height: f32,
}

impl<'a> Tile<'a> {
    pub const fn new(archive: &'a GameArchive, item: &'a ItemStack) -> Self {
        Self {
            archive,
            item,
            width: 0.0,
            height: 0.0,
        }
    }

    pub const fn square(mut self, n: f32) -> Self {
        self.width = n;
        self.height = n;
        self
    }
}

impl<'a> Widget for Tile<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        // let rect = Rect {
        //     min: start,
        //     max: Pos2::new(self.width + start.x, self.height + start.y),
        // };

        // let resp = ui.allocate_rect(
        //     rect,
        //     Sense {
        //         click: false,
        //         drag: false,
        //         focusable: false,
        //     },
        // );

        let resp = ui.allocate_response(
            Vec2::new(self.width, self.height),
            Sense {
                click: false,
                drag: false,
                focusable: false,
            },
        );

        if ui.is_rect_visible(resp.rect) {
            let painter = ui.painter();
            painter.rect(resp.rect, Rounding::none(), Color32::RED, Stroke::none());
        }

        // let (resp, painter) = ui.allocate_painter(
        //     Vec2::new(self.width, self.height),
        //     Sense {
        //         drag: false,
        //         click: false,
        //         focusable: false,
        //     },
        // );

        // let rect = Rect {
        //     min: Pos2::new(0.0, 0.0),
        //     max: Pos2::new(self.width, self.height),
        // };

        resp
    }
}

pub struct CategoryLabel {}

struct ConextMenu {}

impl ConextMenu {}

impl Widget for ConextMenu {
    fn ui(self, ui: &mut Ui) -> Response {
        ui.label("Drop")
    }
}
