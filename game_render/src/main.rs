use std::sync::Arc;

use ash::vk::{CuFunctionNVX, PipelineStageFlags};
use bytemuck::{Pod, Zeroable};
use game_common::components::{Color, Transform};
use game_render::backend::allocator::{GeneralPurposeAllocator, UsageFlags};
use game_render::backend::descriptors::DescriptorSetAllocator;
use game_render::backend::vulkan::{Config, DescriptorSetLayout, Pipeline, Sampler};
use game_render::backend::{
    AccessFlags, AddressMode, BufferUsage, CopyBuffer, DescriptorBinding, DescriptorSetDescriptor,
    FilterMode, FragmentStage, ImageDataLayout, LoadOp, MemoryTypeFlags, PipelineBarriers,
    PipelineDescriptor, PipelineStage, QueueCapabilities, QueueSubmit, RenderPassColorAttachment,
    RenderPassDescriptor, SamplerDescriptor, ShaderStages, StoreOp, SwapchainConfig,
    TextureBarrier, TextureDescriptor, TextureFormat, TextureLayout, VertexStage,
    WriteDescriptorBinding, WriteDescriptorResource, WriteDescriptorResources,
};
use game_render::camera::{Camera, Projection, RenderTarget};
use game_render::entities::Object;
use game_render::light::{DirectionalLight, PointLight, SpotLight};
use game_render::pbr::PbrMaterial;
use game_render::{shape, Renderer};
use game_tasks::TaskPool;
use game_window::windows::{WindowBuilder, WindowState};
use game_window::App;
use glam::{vec2, vec3, vec4, Quat, UVec2, Vec2, Vec3, Vec4};
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use tracing::Instrument;

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
                render_main(window);
            }
            _ => (),
        }
    }

    fn update(&mut self, ctx: game_window::WindowManagerContext<'_>) {}
}

fn render_main(window: WindowState) {
    let mut renderer = Renderer::new().unwrap();
    let mut pool = TaskPool::new(1);

    renderer.create(window.id(), window.clone());

    // let scene_id = renderer.resources().scenes().insert();
    // renderer.resources().cameras().insert(Camera {
    //     transform: Transform {
    //         translation: Vec3::new(10.0, 0.0, 0.0),
    //         ..Default::default()
    //     }
    //     .looking_at(Vec3::ZERO, Vec3::Y),
    //     projection: Projection::default(),
    //     target: RenderTarget::Window(window.id()),
    //     scene: scene_id,
    // });
    // let box_shape = renderer.resources().meshes().insert(
    //     shape::Box {
    //         max_x: 1.0,
    //         min_x: -1.0,
    //         max_y: 1.0,
    //         min_y: -1.0,
    //         min_z: -1.0,
    //         max_z: 1.0,
    //     }
    //     .into(),
    // );
    // let material = renderer
    //     .resources()
    //     .materials()
    //     .insert(PbrMaterial::default());
    // renderer.resources().objects().insert(Object {
    //     transform: Transform::default(),
    //     scene: scene_id,
    //     mesh: box_shape,
    //     material,
    // });

    // renderer.resources().point_lights().insert(PointLight {
    //     transform: Transform {
    //         translation: Vec3::new(0.0, 1.0, 0.0),
    //         ..Default::default()
    //     },
    //     color: Color::WHITE,
    //     intensity: 70.0,
    //     radius: 100.0,
    //     scene: scene_id,
    // });
    // renderer
    //     .resources()
    //     .directional_lights()
    //     .insert(DirectionalLight {
    //         transform: Transform::from_translation(Vec3::splat(1.0))
    //             .looking_at(Vec3::ZERO, Vec3::Y),
    //         scene: scene_id,
    //         color: Color::WHITE,
    //         illuminance: 100_000.0,
    //     });

    loop {
        renderer.render(&pool);
    }
}

// #[repr(C)]
// #[derive(Copy, Clone, Debug, Zeroable, Pod)]
// struct Vertex {
//     pos: Vec4,
//     color: Vec4,
// }

// impl Vertex {
//     const fn new(pos: Vec4, color: Vec4) -> Self {
//         Self { pos, color }
//     }
// }

// static VERTICES: [Vertex; 3] = [
//     Vertex::new(vec4(0.0, -0.5, 0.0, 0.0), vec4(1.0, 1.0, 1.0, 0.0)),
//     Vertex::new(vec4(0.5, 0.5, 0.0, 0.0), vec4(0.0, 1.0, 0.0, 0.0)),
//     Vertex::new(vec4(-0.5, 0.5, 0.0, 0.0), vec4(0.0, 0.0, 1.0, 0.0)),
// ];

// fn vk_main(state: WindowState) {
//     let instance =
//         game_render::backend::vulkan::Instance::new(Config { validation: true }).unwrap();

//     let texture_data = image::load_from_memory(include_bytes!("../../assets/diffuse.png"))
//         .unwrap()
//         .to_rgba8();

//     // let vert_glsl = include_str!("../shader.vert");
//     // let frag_glsl = include_str!("../shader.frag");
//     let vert_spv = include_bytes!("../vert.spv");
//     let frag_spv = include_bytes!("../frag.spv");
//     // let vert_spv = glsl_to_spirv(&vert_glsl, naga::ShaderStage::Vertex);
//     // let frag_spv = glsl_to_spirv(&frag_glsl, naga::ShaderStage::Fragment);

//     for adapter in instance.adapters() {
//         dbg!(adapter.properties());
//         dbg!(&adapter.queue_families());
//         let mem_props = adapter.memory_properties();

//         for queue_family in adapter.queue_families() {
//             if queue_family
//                 .capabilities
//                 .contains(QueueCapabilities::GRAPHICS)
//             {
//                 let device = adapter.create_device(queue_family.id);
//                 let mut queue = device.queue();

//                 let surface = unsafe {
//                     instance
//                         .create_surface(
//                             state.raw_display_handle().unwrap(),
//                             state.raw_window_handle().unwrap(),
//                         )
//                         .unwrap()
//                 };

//                 let mut pool = device.create_command_pool();

//                 let caps = surface.get_capabilities(&device);
//                 dbg!(&caps);

//                 let mut swapchain = surface.create_swapchain(
//                     &device,
//                     SwapchainConfig {
//                         format: caps.formats[0],
//                         present_mode: game_render::backend::PresentMode::Fifo,
//                         image_count: 4,
//                         extent: state.inner_size(),
//                     },
//                     &caps,
//                 );

//                 let mut image_avail = device.create_semaphore();
//                 let mut render_done = device.create_semaphore();

//                 let mut scheduler = Scheduler::new(device.clone(), mem_props);

//                 let mut node = None;

//                 loop {
//                     let img = swapchain.acquire_next_image(&mut image_avail);

//                     let mut ctx = scheduler.queue();
//                     let swapchain_id = unsafe {
//                         ctx.import_texture(
//                             core::mem::transmute(img.texture()),
//                             AccessFlags::COLOR_ATTACHMENT_WRITE,
//                         )
//                     };

//                     let node = node.get_or_insert_with(|| ExampleNode::setup(&mut ctx));
//                     node.render(&mut ctx, &swapchain_id);

//                     let mut encoder = pool.create_encoder().unwrap();

//                     encoder.insert_pipeline_barriers(&PipelineBarriers {
//                         buffer: &[],
//                         texture: &[TextureBarrier {
//                             texture: img.texture(),
//                             src_access: AccessFlags::empty(),
//                             dst_access: AccessFlags::COLOR_ATTACHMENT_WRITE,
//                         }],
//                     });

//                     let cmds = ctx.finish();
//                     let res = scheduler.execute(cmds, &mut encoder);

//                     encoder.insert_pipeline_barriers(&PipelineBarriers {
//                         buffer: &[],
//                         texture: &[TextureBarrier {
//                             texture: img.texture(),
//                             src_access: AccessFlags::COLOR_ATTACHMENT_WRITE,
//                             dst_access: AccessFlags::PRESENT,
//                         }],
//                     });

//                     queue.submit(
//                         std::iter::once(encoder.finish()),
//                         QueueSubmit {
//                             wait: &mut [&mut image_avail],
//                             wait_stage: PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
//                             signal: &mut [&mut render_done],
//                         },
//                     );

//                     img.present(&queue, &render_done);
//                     queue.wait_idle();
//                     unsafe {
//                         pool.reset();
//                     }
//                     drop(res);

//                     if img.suboptimal {
//                         drop(img);
//                         let caps = surface.get_capabilities(&device);
//                         swapchain.recreate(
//                             SwapchainConfig {
//                                 format: caps.formats[0],
//                                 present_mode: game_render::backend::PresentMode::Fifo,
//                                 image_count: 4,
//                                 extent: state.inner_size().clamp(caps.min_extent, caps.max_extent),
//                             },
//                             &caps,
//                         );
//                     }
//                 }
//             }
//         }
//     }
// }

// struct ExampleNode {
//     pipeline: Arc<Pipeline>,
//     vertex_buffer: game_render::graph::ctx::Buffer,
//     texture: game_render::graph::ctx::Texture,
//     layout: Arc<DescriptorSetLayout>,
//     sampler: Arc<Sampler>,
// }

// impl ExampleNode {
//     fn setup(ctx: &mut game_render::graph::ctx::CommandQueue<'_>) -> Self {
//         let vert_spv = include_bytes!("../vert.spv");
//         let frag_spv = include_bytes!("../frag.spv");
//         let vert = unsafe {
//             let data = vert_spv.to_vec();
//             let (prefix, spv, suffix) = data.align_to::<u32>();
//             assert!(prefix.is_empty() && suffix.is_empty());
//             ctx.create_shader(spv)
//         };
//         let frag = unsafe {
//             let data = frag_spv.to_vec();
//             let (prefix, spv, suffix) = data.align_to::<u32>();
//             assert!(prefix.is_empty() && suffix.is_empty());
//             ctx.create_shader(spv)
//         };

//         let descriptor_set_layout = ctx.create_descriptor_set_layout(&DescriptorSetDescriptor {
//             bindings: &[
//                 DescriptorBinding {
//                     binding: 0,
//                     visibility: ShaderStages::VERTEX,
//                     kind: game_render::backend::DescriptorType::Uniform,
//                 },
//                 DescriptorBinding {
//                     binding: 1,
//                     visibility: ShaderStages::FRAGMENT,
//                     kind: game_render::backend::DescriptorType::Sampler,
//                 },
//                 DescriptorBinding {
//                     binding: 2,
//                     visibility: ShaderStages::FRAGMENT,
//                     kind: game_render::backend::DescriptorType::Texture,
//                 },
//             ],
//         });

//         let pipeline = ctx.create_pipeline(&PipelineDescriptor {
//             topology: game_render::backend::PrimitiveTopology::TriangleList,
//             front_face: game_render::backend::FrontFace::Ccw,
//             cull_mode: None,
//             stages: &[
//                 PipelineStage::Vertex(VertexStage { shader: &vert }),
//                 PipelineStage::Fragment(FragmentStage {
//                     shader: &frag,
//                     targets: &[TextureFormat::Bgra8UnormSrgb],
//                 }),
//             ],
//             descriptors: &[&descriptor_set_layout],
//         });

//         let texture_data = image::load_from_memory(include_bytes!("../../assets/diffuse.png"))
//             .unwrap()
//             .to_rgba8();
//         let texture = ctx.create_texture(TextureDescriptor {
//             size: UVec2::new(texture_data.width(), texture_data.height()),
//             format: TextureFormat::Rgba8UnormSrgb,
//             mip_levels: 1,
//         });
//         ctx.write_texture(
//             &texture,
//             &texture_data,
//             ImageDataLayout {
//                 bytes_per_row: 4 * texture_data.width(),
//                 rows_per_image: texture_data.height(),
//             },
//         );

//         let vertex_buffer = ctx.create_buffer(BufferDescriptor {
//             size: size_of::<Vertex>() as u64 * VERTICES.len() as u64,
//         });
//         ctx.write_buffer(&vertex_buffer, bytemuck::cast_slice(&VERTICES));

//         let sampler = ctx.create_sampler(&SamplerDescriptor {
//             min_filter: FilterMode::Linear,
//             mag_filter: FilterMode::Linear,
//             address_mode_u: AddressMode::Repeat,
//             address_mode_v: AddressMode::Repeat,
//             address_mode_w: AddressMode::Repeat,
//         });

//         Self {
//             pipeline: pipeline.into(),
//             texture,
//             vertex_buffer,
//             layout: descriptor_set_layout.into(),
//             sampler: sampler.into(),
//         }
//     }

//     fn render(
//         &self,
//         ctx: &mut game_render::graph::ctx::CommandQueue<'_>,
//         target_view: &game_render::graph::ctx::Texture,
//     ) {
//         let bg = ctx.create_bind_group(BindGroupDescriptor {
//             layout: &self.layout,
//             entries: &[
//                 BindGroupEntry {
//                     binding: 0,
//                     resource: game_render::graph::ctx::BindingResource::Buffer(&self.vertex_buffer),
//                 },
//                 BindGroupEntry {
//                     binding: 1,
//                     resource: game_render::graph::ctx::BindingResource::Sampler(&self.sampler),
//                 },
//                 BindGroupEntry {
//                     binding: 2,
//                     resource: game_render::graph::ctx::BindingResource::Texture(&self.texture),
//                 },
//             ],
//         });

//         let descriptor = game_render::graph::ctx::RenderPassDescriptor {
//             color_attachments: &[game_render::graph::ctx::RenderPassColorAttachment {
//                 texture: target_view,
//                 load_op: LoadOp::Clear([0.0; 4]),
//                 store_op: StoreOp::Store,
//             }],
//         };
//         let mut render_pass = ctx.run_render_pass(&descriptor);

//         render_pass.set_pipeline(&self.pipeline);
//         render_pass.set_bind_group(0, &bg);
//         render_pass.draw(0..3, 0..1);

//         drop(render_pass);
//     }
// }
