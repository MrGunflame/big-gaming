use game_render::texture::Image;
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
    fn size(&self) -> Bounds;

    fn render(&self, ctx: Context<'_>);
}
