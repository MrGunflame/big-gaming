//! The public interface API

use std::any::{Any, TypeId};
use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign};

use bevy::prelude::Resource;
use bevy_ecs::world::World;
use bevy_egui::EguiContext;

#[derive(Resource)]
pub struct InterfaceState {
    widgets: Vec<WidgetCell>,
    capture_pointer: bool,
    capture_keys: bool,
}

impl InterfaceState {
    pub fn new() -> Self {
        Self {
            widgets: Vec::new(),
            capture_pointer: false,
            capture_keys: false,
        }
    }

    pub fn len(&self) -> usize {
        self.widgets.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn render(&mut self, world: &mut World) {
        world.resource_scope::<EguiContext, ()>(|world, mut ctx| {
            let mut ctx = Context {
                ctx: ctx.ctx_mut(),
                world,
                close: false,
            };

            for widget in &mut self.widgets {
                widget.create();

                widget.boxed.render(&mut ctx);
            }
        });
    }

    pub fn push<T>(&mut self, widget: T)
    where
        T: Widget + Any,
    {
        let flags = widget.flags();
        let captures_pointer = flags.intersects(WidgetFlags::CAPTURE_POINTER);
        let captures_keys = flags.intersects(WidgetFlags::CAPTURE_KEYS);

        if !self.capture_pointer && captures_pointer {
            self.capture_pointer = true;
        }

        if !self.capture_keys && captures_keys {
            self.capture_keys = true;
        }

        self.push_boxed(Box::new(widget), TypeId::of::<T>());
    }

    pub fn pop(&mut self) {
        // FIXME: Respect flags
        self.widgets.pop().map(|w| w.destroy());
    }

    fn push_boxed(&mut self, widget: Box<dyn Widget>, type_id: TypeId) {
        self.widgets.push(WidgetCell {
            id: type_id,
            boxed: widget,
            is_created: false,
        });
    }

    pub fn captures_pointer(&self) -> bool {
        self.capture_pointer
    }

    pub fn captures_keys(&self) -> bool {
        self.capture_keys
    }
}

/// A standalone component of the user interface. It may be an interactive window, or a simple
/// non-interctive overlay.
// FIXME: Replace with a raw vtable for external use.
pub trait Widget: Send + Sync + 'static {
    fn flags(&self) -> WidgetFlags {
        WidgetFlags(0)
    }

    fn create(&mut self);
    fn render(&mut self, ctx: &mut Context);
    fn destroy(&mut self);
}

struct WidgetCell {
    id: TypeId,
    boxed: Box<dyn Widget>,
    is_created: bool,
}

impl WidgetCell {
    /// Creates the [`Widget`] if it wasn't already created.
    #[inline]
    fn create(&mut self) {
        if !self.is_created {
            self.boxed.create();
        }
    }

    #[inline]
    fn destroy(mut self) {
        if self.is_created {
            self.boxed.destroy();
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct WidgetFlags(u8);

impl WidgetFlags {
    /// Whether the widget should capture pointer (usually mouse) events.
    ///
    /// If `true` pointer events will be intercepted by the widget and not pass down to any other
    /// widgets or the world.
    pub const CAPTURE_POINTER: Self = Self(1);

    /// Whether the widget should capture key events.
    ///
    /// If `true` key events will be intercepted by the widget and not pass down to any other
    /// widgets or the world.
    pub const CAPTURE_KEYS: Self = Self(1 << 1);

    /// Ignore manual close events by the user. The interface must be destroyed manually.
    pub const IGNORE_CLOSE: Self = Self(1 << 2);

    pub const fn intersects(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }
}

impl BitOr for WidgetFlags {
    type Output = Self;

    #[inline]
    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for WidgetFlags {
    #[inline]
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 = self.0 | rhs.0;
    }
}

impl BitAnd for WidgetFlags {
    type Output = Self;

    #[inline]
    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for WidgetFlags {
    #[inline]
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 = self.0 & rhs.0;
    }
}

pub struct Context<'a> {
    pub ctx: &'a bevy_egui::egui::Context,
    pub world: &'a mut World,

    close: bool,
}

impl<'a> Context<'a> {
    pub fn close(&mut self) {
        self.close = true;
    }
}
