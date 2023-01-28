use bevy::prelude::With;
use bevy_egui::egui::{
    Align, Area, Color32, Layout, Pos2, Rect, Rounding, Sense, Stroke, Ui, Vec2,
};
use common::archive::GameArchive;
use common::components::inventory::Inventory as InventoryComp;
use common::components::items::ItemStack;
use common::components::player::HostPlayer;

use crate::{Context, Widget, WidgetFlags};

#[derive(Clone, Debug, Default)]
pub struct Inventory {}

impl Widget for Inventory {
    fn name(&self) -> &'static str {
        "core::inventory"
    }

    fn flags(&self) -> WidgetFlags {
        WidgetFlags::CAPTURE_POINTER | WidgetFlags::CAPTURE_KEYS
    }

    fn create(&mut self) {}

    fn render(&mut self, ctx: &mut Context) {
        let inventory = ctx
            .world
            .query_filtered::<&InventoryComp, With<HostPlayer>>()
            .single(ctx.world);

        let archive = ctx.world.resource::<GameArchive>();

        Area::new("inventory")
            .fixed_pos(Pos2::new(0.0, 0.0))
            .show(ctx.ctx, |ui| {
                render(ui, archive, inventory);
            });
    }

    fn destroy(&mut self) {}
}

fn render(ui: &mut Ui, archive: &GameArchive, inventory: &InventoryComp) {
    let items: Vec<&ItemStack> = inventory.iter().collect();

    ui.add(Category {
        archive,
        title: "test",
        items: &items,
    });
}

#[derive(Debug)]
struct Category<'a> {
    archive: &'a GameArchive,
    title: &'a str,
    items: &'a [&'a ItemStack],
}

impl<'a> bevy_egui::egui::Widget for Category<'a> {
    fn ui(self, ui: &mut Ui) -> bevy_egui::egui::Response {
        let resp = ui.horizontal_wrapped(|ui| {
            for stack in self.items {
                ui.add(Tile::new(self.archive, stack).square(64.0));
            }
        });

        resp.response
    }
}

struct Tile<'a> {
    archive: &'a GameArchive,
    stack: &'a ItemStack,
    width: f32,
    height: f32,
}

impl<'a> Tile<'a> {
    const fn new(archive: &'a GameArchive, stack: &'a ItemStack) -> Self {
        Self {
            archive,
            stack: stack,
            width: 0.0,
            height: 0.0,
        }
    }

    const fn square(mut self, n: f32) -> Self {
        self.width = n;
        self.height = n;
        self
    }
}

impl<'a> bevy_egui::egui::Widget for Tile<'a> {
    fn ui(self, ui: &mut Ui) -> bevy_egui::egui::Response {
        let item = self.archive.items().get(self.stack.item.id);

        let resp = ui.allocate_response(
            Vec2::new(self.width, self.height),
            Sense {
                click: false,
                focusable: false,
                drag: false,
            },
        );

        if ui.is_rect_visible(resp.rect) {
            let painter = ui.painter();
            painter.rect(resp.rect, Rounding::none(), Color32::RED, Stroke::NONE);
        }

        if let Some(cursor) = resp.hover_pos() {
            TileHover {
                title: item.unwrap().name.as_str(),
            }
            .show(ui, cursor);
        }

        resp
    }
}

struct TileHover<'a> {
    title: &'a str,
}

impl<'a> TileHover<'a> {
    fn show(self, ui: &mut Ui, cursor: Pos2) {
        let rect = Rect {
            min: cursor,
            max: Pos2::new(cursor.x + 32.0, cursor.y + 32.0),
        };

        let mut ui = ui.child_ui(rect, Layout::left_to_right(Align::TOP));
        ui.label(self.title);
    }
}
