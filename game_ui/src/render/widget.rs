use game_input::keyboard::KeyboardInput;
use game_input::mouse::{MouseButtonInput, MouseWheel};
use game_render::texture::Image;
use game_window::events::{CursorEntered, CursorLeft, CursorMoved, ReceivedCharacter};
use glam::Vec2;

use super::style::Bounds;
use super::Rect;

pub struct Context<'a> {
    pub painter: &'a mut Painter,
    pub origin: Vec2,
    pub size: Vec2,
}

pub struct Painter {
    images: Vec<(Vec2, Image)>,
}

impl Painter {
    pub fn draw_image(&mut self, position: Rect, image: Image) {}
}

pub trait Widget {
    /// Returns the minimun and maximum bounds that the widget wants to scale to.
    fn size(&self) -> DrawBounds;

    fn render(&self, ctx: Context<'_>);
}

#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct DrawBounds {
    pub width: f32,
    pub height: f32,
}

#[derive(Clone, Debug)]
pub enum Event {
    CursorMoved(CursorMoved),
    CursorLeft(CursorLeft),
    CursorEntered(CursorEntered),
    MouseButtonInput(MouseButtonInput),
    MouseWheel(MouseWheel),
    KeyboardInput(KeyboardInput),
    ReceivedCharacter(ReceivedCharacter),
}
