use glam::{UVec2, Vec2};
use image::{ImageBuffer, Rgba};

use crate::reactive::Scope;

use super::{Image, Widget};

#[derive(Debug)]
pub struct Plot {
    pub size: UVec2,
    pub points: Vec<Vec2>,
}

impl Widget for Plot {
    fn build(self, cx: &Scope) -> Scope {
        let img = render_graph_to_image(self.size, self.points);

        let root = cx.append(Image::new().image(img));
        root
    }
}

fn render_graph_to_image(size: UVec2, points: Vec<Vec2>) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let mut img = ImageBuffer::new(size.x, size.y);
    if size.x == 0 || size.y == 0 {
        return img;
    }

    for point in points {
        if point.x < 0.0 || point.x > 1.0 || point.y < 0.0 || point.y > 1.0 {
            continue;
        }

        let x = u32::clamp((size.x as f32 * point.x) as u32, 0, size.x - 1);
        let y = u32::clamp((size.y as f32 * point.y) as u32, 0, size.y - 1);
        img.put_pixel(x, y, Rgba([255, 0, 0, 255]));
    }

    img
}
