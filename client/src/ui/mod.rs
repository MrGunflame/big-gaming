mod cursor;
mod events;
mod gun;
mod interfaces;
mod menu;
mod widgets;

pub mod crosshair;
pub mod debug;
pub mod health;

use std::any::{Any, TypeId};

use bevy::ecs::schedule::Stage;
use bevy::prelude::{Component, Plugin, Resource, World};
use bevy_egui::egui::{Context, Sense};
use bevy_egui::{EguiContext, EguiPlugin};

pub struct UiStage;

impl Stage for UiStage {
    fn run(&mut self, world: &mut bevy::prelude::World) {
        world.resource_scope::<InterfaceState, ()>(|world, mut state| {
            state.render(world);
        });
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Component)]
pub enum Focus {
    /// The focus is on the game world.
    World,
    /// The focus is on an interface.
    Interface,
}

/// The user interface plugin.
pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_plugin(EguiPlugin)
            .insert_resource(InterfaceState::new())
            .add_event::<Focus>()
            .add_startup_system(events::register_events)
            .add_system(events::handle_events)
            .add_system(events::toggle_focus)
            .add_system(crosshair::crosshair)
            .add_system(health::health)
            .add_system(gun::gun)
            .add_stage("RenderInterface", UiStage);
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct InterfaceId(u32);

#[derive(Resource)]
pub struct InterfaceState {
    /// The stack of rendered interfaces, rendered in the order they are.
    interfaces: Vec<InterfaceCell>,
    widgets: Vec<InterfaceCell>,
}

impl InterfaceState {
    pub fn new() -> Self {
        Self {
            interfaces: Vec::new(),
            widgets: Vec::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.interfaces.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn push<T>(&mut self, interface: T)
    where
        T: Interface + Send + Sync + 'static,
    {
        self.push_boxed(Box::new(interface), TypeId::of::<T>());
    }

    pub fn contains<T>(&mut self) -> bool
    where
        T: 'static,
    {
        for cell in &self.interfaces {
            if TypeId::of::<T>() == cell.id {
                return true;
            }
        }

        for cell in &self.widgets {
            if TypeId::of::<T>() == cell.id {
                return true;
            }
        }

        false
    }

    pub fn push_default<T>(&mut self)
    where
        T: Interface + Send + Sync + 'static + Default,
    {
        self.push(T::default());
    }

    pub fn remove<T>(&mut self) -> Option<Box<dyn Interface>>
    where
        T: 'static,
    {
        for (i, cell) in self.interfaces.iter().enumerate() {
            if TypeId::of::<T>() == cell.id {
                let mut cell = self.interfaces.remove(i);

                if cell.created {
                    cell.boxed.destroy();
                }

                return Some(cell.boxed);
            }
        }

        for (i, cell) in self.widgets.iter().enumerate() {
            if TypeId::of::<T>() == cell.id {
                let mut cell = self.widgets.remove(i);

                if cell.created {
                    cell.boxed.destroy();
                }

                return Some(cell.boxed);
            }
        }

        None
    }

    pub fn pop(&mut self) -> Option<Box<dyn Interface + Send + Sync + 'static>> {
        self.interfaces.pop().map(|mut cell| {
            if cell.created {
                cell.boxed.destroy();
            }

            cell.boxed
        })
    }

    pub fn render(&mut self, world: &mut World) {
        world.resource_scope::<EguiContext, ()>(|world, mut ctx| {
            for cell in &mut self.widgets {
                if !cell.created {
                    cell.boxed.create();
                    cell.created = true;
                }

                cell.boxed.render(ctx.ctx_mut(), world);
            }

            for cell in &mut self.interfaces {
                if !cell.created {
                    cell.boxed.create();
                    cell.created = true;
                }

                cell.boxed.render(ctx.ctx_mut(), world);
            }
        });
    }

    fn push_boxed(&mut self, boxed: Box<dyn Interface + Send + Sync + 'static>, type_id: TypeId) {
        match boxed.kind() {
            InterfaceKind::Interface => {
                self.interfaces.push(InterfaceCell::new(boxed, type_id));
            }
            InterfaceKind::Widget => {
                self.widgets.push(InterfaceCell::new(boxed, type_id));
            }
        }
    }
}

pub trait Interface: Any {
    fn kind(&self) -> InterfaceKind {
        InterfaceKind::Interface
    }

    fn create(&mut self);
    fn render(&mut self, ctx: &Context, world: &mut World);
    fn destroy(&mut self);
}

struct InterfaceCell {
    id: TypeId,
    boxed: Box<dyn Interface + Send + Sync + 'static>,
    created: bool,
}

impl InterfaceCell {
    fn new(boxed: Box<dyn Interface + Send + Sync + 'static>, type_id: TypeId) -> Self {
        Self {
            id: type_id,
            boxed,
            created: false,
        }
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum InterfaceKind {
    #[default]
    Interface,
    Widget,
}

pub trait SenseExt: private::Sealed {
    fn none() -> Sense;
}

impl SenseExt for Sense {
    fn none() -> Sense {
        Sense {
            click: false,
            drag: false,
            focusable: false,
        }
    }
}

#[doc(hidden)]
impl private::Sealed for Sense {}

mod private {
    pub trait Sealed {}
}
