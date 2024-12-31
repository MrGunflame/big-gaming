use bytemuck::{Pod, Zeroable};
use game_render::backend::descriptors::DescriptorSetAllocator;
use game_render::backend::{
    BufferUsage, DescriptorBinding, DescriptorSetDescriptor, FragmentStage, LoadOp,
    MemoryTypeFlags, PipelineDescriptor, PipelineStage, QueueCapabilities,
    RenderPassColorAttachment, RenderPassDescriptor, ShaderStages, StoreOp, SwapchainConfig,
    VertexStage, WriteDescriptorBinding, WriteDescriptorResource, WriteDescriptorResources,
};
use game_window::windows::{WindowBuilder, WindowState};
use game_window::App;
use glam::{vec2, vec3, vec4, UVec2, Vec2, Vec3, Vec4};
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

#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
struct Vertex {
    pos: Vec4,
    color: Vec4,
}

impl Vertex {
    const fn new(pos: Vec4, color: Vec4) -> Self {
        Self { pos, color }
    }
}

static VERTICES: [Vertex; 3] = [
    Vertex::new(vec4(0.0, -0.5, 0.0, 0.0), vec4(1.0, 1.0, 1.0, 0.0)),
    Vertex::new(vec4(0.5, 0.5, 0.0, 0.0), vec4(0.0, 1.0, 0.0, 0.0)),
    Vertex::new(vec4(-0.5, 0.5, 0.0, 0.0), vec4(0.0, 0.0, 1.0, 0.0)),
];

fn vk_main(state: WindowState) {
    let instance = game_render::backend::vulkan::Instance::new().unwrap();

    let vert_spv = include_bytes!("../vert.spv");
    let frag_spv = include_bytes!("../frag.spv");

    for adapter in instance.adapters() {
        dbg!(adapter.properties());
        dbg!(&adapter.queue_families());
        let mem_props = adapter.memory_properties();

        for queue_family in adapter.queue_families() {
            if queue_family
                .capabilities
                .contains(QueueCapabilities::GRAPHICS)
            {
                let device = adapter.create_device(queue_family.id);
                let mut queue = device.queue();

                let surface = unsafe {
                    instance.create_surface(
                        state.raw_display_handle().unwrap(),
                        state.raw_window_handle().unwrap(),
                    )
                };

                let mut buffer = device.create_buffer(
                    (size_of::<Vertex>() as u64 * VERTICES.len() as u64)
                        .try_into()
                        .unwrap(),
                    BufferUsage::UNIFORM | BufferUsage::TRANSFER_DST,
                );
                let reqs = device.buffer_memory_requirements(&buffer);

                let pad = reqs.padding_needed();

                let host_mem_typ = mem_props
                    .types
                    .iter()
                    .find(|t| {
                        reqs.memory_types.contains(&t.id)
                            && t.flags.contains(MemoryTypeFlags::HOST_VISIBLE)
                            && t.flags.contains(MemoryTypeFlags::DEVICE_LOCAL)
                    })
                    .unwrap();

                let mem = device.allocate_memory(reqs.size.try_into().unwrap(), host_mem_typ.id);
                let mapped_mem = unsafe { device.map_memory(&mem) };
                device.bind_buffer_memory(&mut buffer, mem);

                mapped_mem[..VERTICES.len() * size_of::<Vertex>()]
                    .copy_from_slice(bytemuck::cast_slice(&VERTICES));

                let caps = surface.get_capabilities(&device);
                dbg!(&caps);

                let mut swapchain = surface.create_swapchain(
                    &device,
                    SwapchainConfig {
                        format: caps.formats[0],
                        present_mode: game_render::backend::PresentMode::Fifo,
                        image_count: 4,
                        extent: state.inner_size(),
                    },
                    &caps,
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

                let descriptor_set_layout =
                    device.create_descriptor_layout(&DescriptorSetDescriptor {
                        bindings: &[DescriptorBinding {
                            binding: 0,
                            visibility: ShaderStages::VERTEX,
                            kind: game_render::backend::DescriptorType::Uniform,
                        }],
                    });

                let pipeline = device.create_pipeline(&PipelineDescriptor {
                    topology: game_render::backend::PrimitiveTopology::TriangleList,
                    cull_mode: None,
                    front_face: game_render::backend::FrontFace::Ccw,
                    stages: &[
                        PipelineStage::Vertex(VertexStage { shader: &vert }),
                        PipelineStage::Fragment(FragmentStage {
                            shader: &frag,
                            targets: &[caps.formats[0]],
                        }),
                    ],
                    descriptors: &[&descriptor_set_layout],
                });

                let mut image_avail = device.create_semaphore();
                let mut render_done = device.create_semaphore();

                let mut descriptor_alloc = DescriptorSetAllocator::new(device.clone());

                let mut descriptor_set = unsafe { descriptor_alloc.alloc(&descriptor_set_layout) };

                let buffer_view = buffer.slice(..);
                descriptor_set.raw_mut().update(&WriteDescriptorResources {
                    bindings: &[WriteDescriptorBinding {
                        binding: 0,
                        resource: WriteDescriptorResource::Buffer(&buffer_view),
                    }],
                });

                loop {
                    let img = swapchain.acquire_next_image(&mut image_avail);

                    let mut encoder = pool.create_encoder().unwrap();

                    encoder.emit_pipeline_barrier(
                        img.texture(),
                        ash::vk::ImageLayout::UNDEFINED,
                        ash::vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                        ash::vk::PipelineStageFlags::TOP_OF_PIPE,
                        ash::vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                        ash::vk::AccessFlags::empty(),
                        ash::vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
                    );

                    let view = img.texture().create_view();

                    let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                        color_attachments: &[RenderPassColorAttachment {
                            load_op: LoadOp::Clear([1.0, 1.0, 1.0, 1.0]),
                            store_op: StoreOp::Store,
                            view: &view,
                            layout: ash::vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                            size: img.texture().size(),
                        }],
                    });
                    render_pass.bind_pipeline(&pipeline);

                    render_pass.bind_descriptor_set(0, descriptor_set.raw());

                    render_pass.draw(0..3, 0..1);
                    drop(render_pass);

                    encoder.emit_pipeline_barrier(
                        img.texture(),
                        ash::vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                        ash::vk::ImageLayout::PRESENT_SRC_KHR,
                        ash::vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                        ash::vk::PipelineStageFlags::TOP_OF_PIPE,
                        ash::vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
                        ash::vk::AccessFlags::empty(),
                    );

                    queue.submit(
                        &[encoder.finish()],
                        &mut image_avail,
                        ash::vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                        &mut render_done,
                    );

                    img.present(&queue, &render_done);
                    queue.wait_idle();
                    drop(view);
                    unsafe {
                        pool.reset();
                    }

                    if img.suboptimal {
                        drop(img);
                        let caps = surface.get_capabilities(&device);
                        swapchain.recreate(
                            SwapchainConfig {
                                format: caps.formats[0],
                                present_mode: game_render::backend::PresentMode::Fifo,
                                image_count: 4,
                                extent: state.inner_size().clamp(caps.min_extent, caps.max_extent),
                            },
                            &caps,
                        );
                    }
                }
            }
        }
    }
}
