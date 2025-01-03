use ash::vk::PipelineStageFlags;
use bytemuck::{Pod, Zeroable};
use game_render::backend::descriptors::DescriptorSetAllocator;
use game_render::backend::shader::glsl_to_spirv;
use game_render::backend::{
    AccessFlags, AddressMode, BufferUsage, CopyBuffer, DescriptorBinding, DescriptorSetDescriptor,
    FilterMode, FragmentStage, ImageDataLayout, LoadOp, MemoryTypeFlags, PipelineBarriers,
    PipelineDescriptor, PipelineStage, QueueCapabilities, QueueSubmit, RenderPassColorAttachment,
    RenderPassDescriptor, SamplerDescriptor, ShaderStages, StoreOp, SwapchainConfig,
    TextureBarrier, TextureDescriptor, TextureFormat, TextureLayout, VertexStage,
    WriteDescriptorBinding, WriteDescriptorResource, WriteDescriptorResources,
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

    let texture_data = image::load_from_memory(include_bytes!("../../assets/diffuse.png"))
        .unwrap()
        .to_rgba8();

    // let vert_glsl = include_str!("../shader.vert");
    // let frag_glsl = include_str!("../shader.frag");
    let vert_spv = include_bytes!("../vert.spv");
    let frag_spv = include_bytes!("../frag.spv");
    // let vert_spv = glsl_to_spirv(&vert_glsl, naga::ShaderStage::Vertex);
    // let frag_spv = glsl_to_spirv(&frag_glsl, naga::ShaderStage::Fragment);

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

                let mut pool = device.create_command_pool();

                let mut texture = device.create_texture(&TextureDescriptor {
                    size: UVec2::new(texture_data.width(), texture_data.height()),
                    format: TextureFormat::R8G8B8A8UnormSrgb,
                    mip_levels: 1,
                });
                {
                    let mut encoder = pool.create_encoder().unwrap();

                    let mut staging_buffer = device.create_buffer(
                        ((texture_data.height() * texture_data.width() * 4) as u64)
                            .try_into()
                            .unwrap(),
                        BufferUsage::TRANSFER_DST | BufferUsage::TRANSFER_SRC,
                    );
                    let texture_reqs = device.image_memory_requirements(&texture);
                    let buffer_reqs = device.buffer_memory_requirements(&staging_buffer);

                    let host_mem_typ_tex = mem_props
                        .types
                        .iter()
                        .find(|t| {
                            texture_reqs.memory_types.contains(&t.id)
                                && t.flags.contains(MemoryTypeFlags::DEVICE_LOCAL)
                        })
                        .unwrap();
                    let host_mem_typ_buf = mem_props
                        .types
                        .iter()
                        .find(|t| {
                            buffer_reqs.memory_types.contains(&t.id)
                                && t.flags.contains(MemoryTypeFlags::HOST_VISIBLE)
                                && t.flags.contains(MemoryTypeFlags::HOST_COHERENT)
                        })
                        .unwrap();

                    let tex_mem = device.allocate_memory(texture_reqs.size, host_mem_typ_tex.id);
                    let mut buf_mem = device.allocate_memory(buffer_reqs.size, host_mem_typ_buf.id);

                    unsafe {
                        buf_mem.map(..).copy_from_slice(&texture_data);
                    }

                    unsafe {
                        device.bind_buffer_memory(&mut staging_buffer, buf_mem.slice(..));
                    }
                    unsafe {
                        device.bind_texture_memory(&mut texture, tex_mem.slice(..));
                    }

                    encoder.insert_pipeline_barriers(&PipelineBarriers {
                        texture: &[TextureBarrier {
                            texture: &texture,
                            // old_layout: TextureLayout::Undefined,
                            // new_layout: TextureLayout::TransferDst,
                            // src_access_flags: ash::vk::AccessFlags2::empty(),
                            // dst_access_flags: ash::vk::AccessFlags2::TRANSFER_WRITE,
                            src_access: AccessFlags::empty(),
                            dst_access: AccessFlags::TRANSFER_WRITE,
                        }],
                        buffer: &[],
                    });

                    encoder.copy_buffer_to_texture(
                        CopyBuffer {
                            buffer: &staging_buffer,
                            offset: 0,
                            layout: ImageDataLayout {
                                bytes_per_row: 4 * texture_data.width(),
                                rows_per_image: texture_data.height(),
                            },
                        },
                        &texture,
                    );

                    encoder.insert_pipeline_barriers(&PipelineBarriers {
                        buffer: &[],
                        texture: &[TextureBarrier {
                            texture: &mut texture,
                            // old_layout: TextureLayout::TransferDst,
                            // new_layout: TextureLayout::ShaderRead,
                            // src_access_flags: ash::vk::AccessFlags2::TRANSFER_WRITE,
                            // dst_access_flags: ash::vk::AccessFlags2::SHADER_READ,
                            src_access: AccessFlags::TRANSFER_WRITE,
                            dst_access: AccessFlags::SHADER_READ,
                        }],
                    });

                    queue.submit(
                        std::iter::once(encoder.finish()),
                        QueueSubmit {
                            wait: &mut [],
                            wait_stage: PipelineStageFlags::empty(),
                            signal: &mut [],
                        },
                    );
                    queue.wait_idle();
                    unsafe {
                        pool.reset();
                    }
                }

                let texture_view = texture.create_view();

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

                let mut mem =
                    device.allocate_memory(reqs.size.try_into().unwrap(), host_mem_typ.id);
                let mapped_mem = unsafe { mem.map(..) };
                mapped_mem[..VERTICES.len() * size_of::<Vertex>()]
                    .copy_from_slice(bytemuck::cast_slice(&VERTICES));
                unsafe {
                    device.bind_buffer_memory(&mut buffer, mem.slice(..));
                }

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

                let sampler = device.create_sampler(&SamplerDescriptor {
                    min_filter: FilterMode::Linear,
                    mag_filter: FilterMode::Linear,
                    address_mode_u: AddressMode::Repeat,
                    address_mode_v: AddressMode::Repeat,
                    address_mode_w: AddressMode::Repeat,
                });

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

                let descriptor_set_layout =
                    device.create_descriptor_layout(&DescriptorSetDescriptor {
                        bindings: &[
                            DescriptorBinding {
                                binding: 0,
                                visibility: ShaderStages::VERTEX,
                                kind: game_render::backend::DescriptorType::Uniform,
                            },
                            DescriptorBinding {
                                binding: 1,
                                visibility: ShaderStages::FRAGMENT,
                                kind: game_render::backend::DescriptorType::Sampler,
                            },
                            DescriptorBinding {
                                binding: 2,
                                visibility: ShaderStages::FRAGMENT,
                                kind: game_render::backend::DescriptorType::Texture,
                            },
                        ],
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
                    bindings: &[
                        WriteDescriptorBinding {
                            binding: 0,
                            resource: WriteDescriptorResource::Buffer(&buffer_view),
                        },
                        WriteDescriptorBinding {
                            binding: 1,
                            resource: WriteDescriptorResource::Sampler(&sampler),
                        },
                        WriteDescriptorBinding {
                            binding: 2,
                            resource: WriteDescriptorResource::Texture(&texture_view),
                        },
                    ],
                });

                loop {
                    let img = swapchain.acquire_next_image(&mut image_avail);

                    let mut encoder = pool.create_encoder().unwrap();

                    encoder.insert_pipeline_barriers(&PipelineBarriers {
                        buffer: &[],
                        texture: &[TextureBarrier {
                            texture: img.texture(),
                            // src_access_flags: ash::vk::AccessFlags2::empty(),
                            // dst_access_flags: ash::vk::AccessFlags2::COLOR_ATTACHMENT_WRITE,
                            // old_layout: TextureLayout::Undefined,
                            // new_layout: TextureLayout::ColorAttachment,
                            src_access: AccessFlags::empty(),
                            dst_access: AccessFlags::COLOR_ATTACHMENT_WRITE,
                        }],
                    });

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

                    encoder.insert_pipeline_barriers(&PipelineBarriers {
                        buffer: &[],
                        texture: &[TextureBarrier {
                            texture: img.texture(),
                            // src_access_flags: ash::vk::AccessFlags2::COLOR_ATTACHMENT_WRITE,
                            // dst_access_flags: ash::vk::AccessFlags2::empty(),
                            // old_layout: TextureLayout::ColorAttachment,
                            // new_layout: TextureLayout::Present,
                            src_access: AccessFlags::COLOR_ATTACHMENT_WRITE,
                            dst_access: AccessFlags::PRESENT,
                        }],
                    });

                    queue.submit(
                        std::iter::once(encoder.finish()),
                        QueueSubmit {
                            wait: &mut [&mut image_avail],
                            wait_stage: PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                            signal: &mut [&mut render_done],
                        },
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
