mod tests;

use clap::{Parser, Subcommand};
use game_render::camera::RenderTarget;
use game_render::scene::RendererScene;
use game_render::texture::RenderTexture;
use game_render::Renderer;
use game_tasks::TaskPool;
use glam::UVec2;
use image::{ColorType, ImageBuffer, Rgba};
use tests::Options;
use tokio::sync::oneshot;
use tokio::sync::oneshot::error::TryRecvError;

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The render width.
    #[arg(long, default_value_t = 4096)]
    width: u32,

    /// The render height.
    #[arg(long, default_value_t = 4096)]
    height: u32,

    #[command(subcommand)]
    cmd: Command,
}

#[derive(Copy, Clone, Debug, Subcommand)]
enum Command {
    Generate,
    Test,
}

fn main() {
    let args = Args::parse();

    let options = Options {
        cmd: args.cmd,
        size: UVec2::new(args.width, args.height),
    };

    tests::run_tests(options);
}

struct Harness {
    name: &'static str,
    setup: Box<dyn Fn(&mut RendererScene<'_>, RenderTarget)>,
}

impl Harness {
    fn new<F>(name: &'static str, setup: F) -> Self
    where
        F: Fn(&mut RendererScene<'_>, RenderTarget) + 'static,
    {
        Self {
            name,
            setup: Box::new(setup),
        }
    }

    fn run(&mut self, size: UVec2) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
        let pool = TaskPool::new(1);
        let mut renderer = Renderer::new().unwrap();

        let id = renderer.create_render_texture(RenderTexture { size });

        let mut scene = renderer.scene_mut(id.into()).unwrap();

        (self.setup)(&mut scene, RenderTarget::Image(id));

        let fut = renderer.read_gpu_texture(id);

        let (tx, mut rx) = oneshot::channel();

        std::thread::spawn(move || {
            let data = futures_lite::future::block_on(fut);

            let mut buffer = ImageBuffer::new(size.x, size.y);
            assert_eq!(data.len() as u32, size.x * size.y * 4);
            for (index, block) in data.chunks(4).enumerate() {
                let x = index as u32 % size.x;
                let y = index as u32 / size.x;

                let pixel = Rgba([block[0], block[1], block[2], block[3]]);
                buffer.put_pixel(x, y, pixel);
            }

            tx.send(buffer).unwrap();
        });

        let buffer = loop {
            match rx.try_recv() {
                Ok(buffer) => break buffer,
                Err(TryRecvError::Closed) => panic!("channel closed before buffer was read"),
                Err(TryRecvError::Empty) => renderer.render(&pool),
            }
        };

        buffer
    }
}

fn load_sample(test_name: &'static str) -> Option<ImageBuffer<Rgba<u8>, Vec<u8>>> {
    let mut path = std::env::current_dir().unwrap();
    path.push("samples");
    path.push(format!("{}.png", test_name));

    if !path.try_exists().unwrap() {
        return None;
    }

    let img = image::open(path).unwrap();
    Some(img.into_rgba8())
}

fn store_sample(test_name: &'static str, buf: ImageBuffer<Rgba<u8>, Vec<u8>>) {
    let mut path = std::env::current_dir().unwrap();
    path.push("samples");

    if !path.try_exists().unwrap() {
        std::fs::create_dir(&path).unwrap();
    }

    path.push(format!("{}.png", test_name));

    image::save_buffer(path, &buf, buf.width(), buf.height(), ColorType::Rgba8).unwrap();
}
