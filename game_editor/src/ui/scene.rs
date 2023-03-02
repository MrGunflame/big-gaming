use std::collections::VecDeque;

use bevy::prelude::{DespawnRecursiveExt, Entity, With, World};
use bevy_egui::egui::panel::Side;
use bevy_egui::egui::{SidePanel, Ui};
use game_ui::{Context, Widget};

use crate::world::{EntityOptions, Selected};

#[derive(Clone, Debug, Default)]
pub struct SceneHierarchy {}

impl Widget for SceneHierarchy {
    fn name(&self) -> &'static str {
        "editor::scene"
    }

    fn render(&mut self, ctx: &mut Context) {
        SidePanel::new(Side::Right, "scene").show(ctx.ctx, |ui| {
            render(self, ui, ctx.world);
        });
    }
}

fn render(scene: &mut SceneHierarchy, ui: &mut Ui, world: &mut World) {
    let mut query = world.query::<(Entity, &mut EntityOptions)>();

    let mut despawn = Vec::new();

    let mut commands = EntityCommands::new();

    for (entity, mut options) in &mut query.iter_mut(world) {
        let mut node = Node {
            entity,
            options: *options,
            commands: &mut commands,
        };

        node.show(ui);

        while let Some(cmd) = commands.pop() {
            match cmd {
                Command::Destroy => {
                    despawn.push(entity);
                }
                Command::Hide => {
                    options.hidden ^= true;
                }
                Command::Select { exclusive } => {
                    options.selected = true;
                }
            }
        }
    }

    for entity in despawn {
        world.entity_mut(entity).despawn_recursive();
    }
}

#[derive(Debug)]
struct Node<'a> {
    entity: Entity,
    options: EntityOptions,
    commands: &'a mut EntityCommands,
}

impl<'a> Node<'a> {
    fn show(&mut self, ui: &mut Ui) {
        let node = ui.button(format!("{:?}", self.entity));

        if node.clicked() {
            self.commands.select(false);
        }
    }
}

#[derive(Clone, Debug)]
struct EntityCommands {
    queue: VecDeque<Command>,
}

impl EntityCommands {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }

    pub fn hide(&mut self) {
        self.queue.push_back(Command::Hide);
    }

    pub fn destroy(&mut self) {
        self.queue.push_back(Command::Destroy);
    }

    pub fn select(&mut self, exclusive: bool) {
        self.queue.push_back(Command::Select { exclusive });
    }

    pub fn pop(&mut self) -> Option<Command> {
        self.queue.pop_front()
    }
}

#[derive(Clone, Debug)]
enum Command {
    Destroy,
    Hide,
    Select { exclusive: bool },
}
