use game_render::backend::{
    FragmentStage, LoadOp, PipelineDescriptor, PipelineStage, QueueCapabilities,
    RenderPassColorAttachment, RenderPassDescriptor, StoreOp, SwapchainConfig, VertexStage,
};
use game_window::windows::{WindowBuilder, WindowState};
use game_window::App;
use glam::UVec2;
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

    let vert_spv = include_bytes!("../vert.spv");
    let frag_spv = include_bytes!("../frag.spv");

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

                let mut swapchain = surface.create_swapchain(
                    &device,
                    SwapchainConfig {
                        format: game_render::backend::TextureFormat::R8G8B8A8UnormSrgb,
                        present_mode: game_render::backend::PresentMode::Fifo,
                        image_count: 4,
                        extent: state.inner_size(),
                    },
                );

                let vert = unsafe {
                    let data = vert_spv.to_vec();
                    let (prefix, spv, suffix) = data.align_to::<u32>();
                    assert!(prefix.is_empty() && suffix.is_empty());
                    device.create_shader(spv)
                };
                let frag = unsafe {
                    let data = frag_spv.to_vec();
                    let (prefix, spv, suffix) = data.align_to::<u32>();
                    assert!(prefix.is_empty() && suffix.is_empty());
                    device.create_shader(spv)
                };

                let mut pool = device.create_command_pool();

                let pipeline = device.create_pipeline(&PipelineDescriptor {
                    stages: &[
                        PipelineStage::Vertex(VertexStage { shader: &vert }),
                        PipelineStage::Fragment(FragmentStage { shader: &frag }),
                    ],
                });

                let image_avail = device.create_semaphore();
                let render_done = device.create_semaphore();

                let img = swapchain.acquire_next_image(&image_avail);

                let mut encoder = pool.create_encoder();

                encoder.emit_pipeline_barrier(
                    swapchain.images[img as usize],
                    ash::vk::ImageLayout::UNDEFINED,
                    ash::vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                    ash::vk::PipelineStageFlags::TOP_OF_PIPE,
                    ash::vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                    ash::vk::AccessFlags::empty(),
                    ash::vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
                );

                let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                    color_attachments: &[RenderPassColorAttachment {
                        load_op: LoadOp::Clear([1.0, 1.0, 1.0, 1.0]),
                        store_op: StoreOp::Store,
                        view: swapchain.image_views[img as usize],
                        layout: ash::vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                        size: swapchain.extent,
                    }],
                });
                render_pass.bind_pipeline(&pipeline);
                render_pass.draw(0..3, 0..1);
                drop(render_pass);

                encoder.emit_pipeline_barrier(
                    swapchain.images[img as usize],
                    ash::vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                    ash::vk::ImageLayout::PRESENT_SRC_KHR,
                    ash::vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                    ash::vk::PipelineStageFlags::TOP_OF_PIPE,
                    ash::vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
                    ash::vk::AccessFlags::empty(),
                );

                queue.submit(
                    &[encoder.finish()],
                    &image_avail,
                    ash::vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                    &render_done,
                );

                swapchain.present(&queue, img, &render_done);

                std::thread::park();
            }
        }
    }
}
