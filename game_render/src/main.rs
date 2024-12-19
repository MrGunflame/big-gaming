use game_render::backend::{QueueCapabilities, SwapchainConfig};
use game_window::windows::{WindowBuilder, WindowState};
use game_window::App;
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};

fn main() {
    let mut manager = game_window::WindowManager::new();
    let id = manager.windows_mut().spawn(WindowBuilder::new());
    manager.run(MyApp {});
}

struct MyApp {}

impl App for MyApp {
    fn handle_event(
        &mut self,
        ctx: game_window::WindowManagerContext<'_>,
        event: game_window::events::WindowEvent,
    ) {
        match event {
            game_window::events::WindowEvent::WindowCreated(id) => {
                let window = ctx.windows.state(id.window).unwrap();
                vk_main(window);
            }
            _ => (),
        }
    }

    fn update(&mut self, ctx: game_window::WindowManagerContext<'_>) {}
}

fn vk_main(state: WindowState) {
    let instance = game_render::backend::vulkan::Instance::new().unwrap();

    for adapter in instance.adapters() {
        dbg!(adapter.properties());
        dbg!(&adapter.queue_families());

        for queue_family in adapter.queue_families() {
            if queue_family
                .capabilities
                .contains(QueueCapabilities::GRAPHICS)
            {
                let device = adapter.create_device(queue_family.id);
                let queue = device.queue();

                let surface = instance.create_surface(
                    state.raw_display_handle().unwrap(),
                    state.raw_window_handle().unwrap(),
                );

                let caps = surface.get_capabilities(&device);
                dbg!(&caps);

                let swapchain = surface.create_swapchain(
                    &device,
                    SwapchainConfig {
                        format: game_render::backend::TextureFormat::R8G8B8A8UnormSrgb,
                        present_mode: game_render::backend::PresentMode::Fifo,
                        image_count: 4,
                        extent: state.inner_size(),
                    },
                );
            }
        }
    }
}
