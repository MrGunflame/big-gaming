use criterion::{black_box, criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion};
use game_ui::render::computed_style::ComputedStyle;
use game_ui::render::image::apply_background;
use game_ui::style::{Background, Padding, Size, Style};
use glam::Vec2;
use image::{ImageBuffer, Rgba};

fn create_image(size: u32) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    ImageBuffer::from_pixel(size, size, Rgba([123, 65, 249, 198]))
}

const TEST_SIZES: &[u32] = &[128, 256, 512, 1024, 2048, 4096];

fn run_bench(c: &mut Criterion) {
    for (name, style) in [
        (
            "background none/no padding",
            Style {
                background: Background::None,
                padding: Padding::NONE,
                ..Default::default()
            },
        ),
        (
            "background none/padding",
            Style {
                background: Background::None,
                padding: Padding::splat(Size::Pixels(2.0)),
                ..Default::default()
            },
        ),
        (
            "background color/no padding",
            Style {
                background: Background::Color(Rgba([123, 124, 125, 126])),
                padding: Padding::NONE,
                ..Default::default()
            },
        ),
        (
            "background color/padding",
            Style {
                background: Background::Color(Rgba([123, 124, 125, 126])),
                padding: Padding::splat(Size::Pixels(2.0)),
                ..Default::default()
            },
        ),
        (
            "background image (128x128) /no padding",
            Style {
                background: Background::Image(ImageBuffer::from_pixel(
                    128,
                    128,
                    Rgba([123, 124, 125, 126]),
                )),
                padding: Padding::NONE,
                ..Default::default()
            },
        ),
        (
            "background image (128x128)/padding",
            Style {
                background: Background::Image(ImageBuffer::from_pixel(
                    128,
                    128,
                    Rgba([123, 124, 125, 126]),
                )),
                padding: Padding::splat(Size::Pixels(2.0)),
                ..Default::default()
            },
        ),
    ] {
        exec_bench(c, name, style);
    }
}

fn exec_bench(c: &mut Criterion, name: &str, style: Style) {
    let viewport = Vec2::splat(1000.0);
    let style = ComputedStyle::new(style, viewport);

    let mut group = c.benchmark_group(name);

    for size in TEST_SIZES {
        let id = BenchmarkId::new(stringify!(apply_background), size);

        group.bench_with_input(id, &size, |b, &size| {
            b.iter_batched_ref(
                || create_image(*size),
                |image| {
                    black_box(apply_background(image, &style));
                },
                BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

criterion_group!(benches, run_bench);
criterion_main!(benches);
