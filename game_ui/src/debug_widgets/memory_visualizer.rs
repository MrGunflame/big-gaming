use std::sync::Arc;

use game_render::statistics::{AllocationKind, MemoryBlock, Statistics};
use glam::UVec2;
use image::{ImageBuffer, Rgba};

use crate::runtime::Context;
use crate::widgets::{Container, Image, Text, Widget};

#[derive(Clone, Debug)]
pub struct MemoryVisualizer {
    pub size: UVec2,
    pub stats: Arc<Statistics>,
}

impl Widget for MemoryVisualizer {
    fn mount(self, parent: &Context) -> Context {
        let mem = self.stats.memory.read();

        let root = Container::new().mount(parent);

        for (_, block) in mem.blocks.iter() {
            let mut text = format!(
                "Size: {} Used: {} Usage Ratio: {:.2} Allocations: {}",
                bytes_to_human_readable(block.size),
                bytes_to_human_readable(block.used),
                block.used as f64 / block.size as f64,
                block.allocs.len()
            );

            if block.device_local {
                text.push_str(" DEVICE_LOCAL");
            }

            if block.host_visible {
                text.push_str(" HOST_VISIBLE");
            }

            if block.dedicated {
                text.push_str(" DEDICATED");
            }

            Text::new(text).mount(&root);

            let img = draw_block(self.size, block);
            Image::new().image(img).mount(&root);
        }

        root
    }
}

const BLOCK_HEIGHT: u32 = 32;
const PADDING_BETWEEN_BLOCKS: u32 = 4;

const COLOR_UNUSED: Rgba<u8> = Rgba([0x8e, 0x8e, 0x8e, 0xff]);
const COLOR_BUFFER: Rgba<u8> = Rgba([0xff, 0x00, 0x00, 0xff]);
const COLOR_TEXTURE: Rgba<u8> = Rgba([0x00, 0x00, 0xff, 0xff]);

fn draw_block(size: UVec2, block: &MemoryBlock) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let mut img = ImageBuffer::new(size.x, BLOCK_HEIGHT + PADDING_BETWEEN_BLOCKS);

    let block_px = u64::from(BLOCK_HEIGHT) * u64::from(size.x);
    let bytes_per_px = block.size.div_ceil(block_px);

    for x in 0..size.x {
        for y in 0..BLOCK_HEIGHT {
            *img.get_pixel_mut(x, y) = COLOR_UNUSED;
        }
    }

    for alloc in block.allocs.values() {
        let start_px = alloc.offset / bytes_per_px;
        let end_px = (alloc.offset + alloc.size).div_ceil(bytes_per_px);

        for px in start_px as u32..end_px as u32 {
            let x = px / BLOCK_HEIGHT;
            let y = px % BLOCK_HEIGHT;

            let color = match alloc.kind {
                AllocationKind::Buffer => COLOR_BUFFER,
                AllocationKind::Texture => COLOR_TEXTURE,
            };

            *img.get_pixel_mut(x, y) = color;
        }
    }

    img
}

fn bytes_to_human_readable(mut bytes: u64) -> String {
    for unit in ["", "KiB"] {
        if bytes < 1024 {
            return format!("{} {}", bytes, unit);
        }

        bytes /= 1024;
    }

    format!("{} MiB", bytes)
}
