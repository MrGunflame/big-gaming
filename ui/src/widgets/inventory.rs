use bevy::prelude::With;
use bevy_egui::egui::{
    Align, Area, Color32, Layout, PointerButton, Pos2, Rect, Rounding, Sense, Stroke, Ui, Vec2,
};
use common::archive::GameArchive;
use common::components::inventory::Inventory as InventoryComp;
use common::components::items::ItemStack;
use common::components::player::HostPlayer;

use crate::{Context, Widget, WidgetFlags};

#[derive(Clone, Debug, Default)]
pub struct Inventory {
    /// The index of the selected [`Tile`], if any.
    ///
    /// If a tile is selected, hover tooltips for other tiles are disabled and the context menu
    /// for the selected tile is shown.
    selected_tile: Option<(usize, Pos2)>,
}

impl Widget for Inventory {
    fn name(&self) -> &'static str {
        "core::inventory"
    }

    fn flags(&self) -> WidgetFlags {
        WidgetFlags::CAPTURE_POINTER | WidgetFlags::CAPTURE_KEYS
    }

    fn render(&mut self, ctx: &mut Context) {
        let inventory = ctx
            .world
            .query_filtered::<&InventoryComp, With<HostPlayer>>()
            .single(ctx.world);

        let archive = ctx.world.resource::<GameArchive>();

        Area::new("inventory")
            .fixed_pos(Pos2::new(0.0, 0.0))
            .show(ctx.ctx, |ui| {
                render(self, ui, archive, inventory);
            });
    }
}

fn render(state: &mut Inventory, ui: &mut Ui, archive: &GameArchive, inventory: &InventoryComp) {
    let items: Vec<&ItemStack> = inventory.iter().collect();

    ui.add(Category {
        state,
        archive,
        title: "test",
        items: &items,
    });
}

#[derive(Debug)]
struct Category<'a> {
    state: &'a mut Inventory,
    archive: &'a GameArchive,
    title: &'a str,
    items: &'a [&'a ItemStack],
}

impl<'a> bevy_egui::egui::Widget for Category<'a> {
    fn ui(self, ui: &mut Ui) -> bevy_egui::egui::Response {
        let resp = ui.horizontal_wrapped(|ui| {
            for (index, stack) in self.items.iter().enumerate() {
                ui.add(Tile::new(index, self.state, self.archive, stack).square(64.0));
            }
        });

        resp.response
    }
}

struct Tile<'a> {
    index: usize,
    state: &'a mut Inventory,
    archive: &'a GameArchive,
    stack: &'a ItemStack,
    width: f32,
    height: f32,
}

impl<'a> Tile<'a> {
    fn new(
        index: usize,
        state: &'a mut Inventory,
        archive: &'a GameArchive,
        stack: &'a ItemStack,
    ) -> Self {
        Self {
            index,
            state,
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

        let resp = ui.allocate_response(Vec2::new(self.width, self.height), Sense::click());

        if ui.is_rect_visible(resp.rect) {
            let painter = ui.painter();
            painter.rect(resp.rect, Rounding::none(), Color32::RED, Stroke::NONE);
        }

        if let Some((index, pos)) = self.state.selected_tile {
            if self.index == index {
                ContextMenu {
                    state: self.state,
                    rect: Rect {
                        min: pos,
                        max: Pos2::new(pos.x + 32.0, pos.y + 32.0),
                    },
                }
                .show(ui);
            }
        // Only show hover when NO tile is selected.
        } else {
            if let Some(cursor) = resp.hover_pos() {
                if resp.clicked_by(PointerButton::Secondary) {
                    self.state.selected_tile = Some((self.index, cursor));
                } else {
                    TileHover {
                        title: item.unwrap().name.as_str(),
                    }
                    .show(ui, cursor);
                }
            }
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

/// The (right-click) context menu for a tile.
struct ContextMenu<'a> {
    state: &'a mut Inventory,
    rect: Rect,
}

impl<'a> ContextMenu<'a> {
    fn show(self, ui: &mut Ui) {
        let mut ui = ui.child_ui(self.rect, Layout::left_to_right(Align::TOP));

        if ui.label("Drop").clicked_elsewhere() {
            self.state.selected_tile = None;
        }
    }
}
