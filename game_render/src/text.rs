use std::borrow::BorrowMut;
use std::num::NonZeroU32;

use ab_glyph::{
    point, Font, FontRef, Glyph, Outline, OutlinedGlyph, Point, PxScale, PxScaleFactor, Rect,
    ScaleFont,
};
use bytemuck::{Pod, Zeroable};
use glam::Vec2;
use image::{ImageBuffer, Luma, LumaA, Rgba, RgbaImage};
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{
    AddressMode, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType, BlendState,
    Buffer, BufferAddress, BufferUsages, ColorTargetState, ColorWrites, Device, Extent3d, Face,
    FilterMode, FragmentState, ImageCopyTexture, ImageDataLayout, MultisampleState, Origin3d,
    PipelineLayoutDescriptor, PolygonMode, PrimitiveState, PrimitiveTopology, Queue, RenderPass,
    RenderPipeline, RenderPipelineDescriptor, Sampler, SamplerBindingType, SamplerDescriptor,
    ShaderModuleDescriptor, ShaderStages, SurfaceConfiguration, Texture, TextureAspect,
    TextureDescriptor, TextureDimension, TextureFormat, TextureSampleType, TextureUsages,
    TextureViewDescriptor, TextureViewDimension, VertexAttribute, VertexBufferLayout, VertexFormat,
    VertexState, VertexStepMode,
};

use crate::image::debug_border;
use crate::layout::Layout;
use crate::ui::{BuildPrimitiveElement, PrimitiveElement, UiPipeline};
use crate::State;

// pub struct TextPipeline {
//     layout: BindGroupLayout,
//     sampler: Sampler,
//     pipeline: RenderPipeline,
//     buffer: Option<Texture>,
//     vertex_buffer: Buffer,
//     index_buffer: Buffer,
//     size: Vec2,
// }

#[derive(Clone, Debug)]
pub struct Text {
    pub position: Vec2,
    pub text: String,
    pub size: f32,
}

impl Text {
    pub fn dimensions(&self) -> Vec2 {
        let image = render_to_texture(&self.text);
        Vec2::new(image.width() as f32, image.height() as f32)
    }
}

impl BuildPrimitiveElement for Text {
    fn build(
        &self,
        layout: Layout,
        pipeline: &UiPipeline,
        device: &Device,
        queue: &Queue,
        size: Vec2,
    ) -> PrimitiveElement {
        let mut image = render_to_texture(&self.text);

        // Note that the text size and contents themselves define the size
        // of a `Text` element. Overflowing in the x direction will attempt
        // to wrap at the specified width, overflowing in the y direction
        // will cut any exceeding content.
        let start = crate::layout::remap(layout.position, size);
        let end = crate::layout::remap(
            layout.position + Vec2::new(layout.width, layout.height),
            size,
        );

        debug_border(&mut image);

        PrimitiveElement::new(
            pipeline,
            device,
            queue,
            start,
            end,
            &image,
            [1.0, 1.0, 1.0, 1.0],
        )
    }
}

// impl TextPipeline {
//     pub fn new(device: &Device, queue: &Queue, config: &SurfaceConfiguration, size: Vec2) -> Self {
//         let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
//             entries: &[
//                 BindGroupLayoutEntry {
//                     binding: 0,
//                     visibility: ShaderStages::FRAGMENT,
//                     ty: BindingType::Texture {
//                         sample_type: TextureSampleType::Float { filterable: true },
//                         view_dimension: TextureViewDimension::D2,
//                         multisampled: false,
//                     },
//                     count: None,
//                 },
//                 BindGroupLayoutEntry {
//                     binding: 1,
//                     visibility: ShaderStages::FRAGMENT,
//                     ty: BindingType::Sampler(SamplerBindingType::Filtering),
//                     count: None,
//                 },
//             ],
//             label: Some("texture_bind_group_layout"),
//         });

//         let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
//             label: Some("text_pipeline_layout"),
//             bind_group_layouts: &[&bind_group_layout],
//             push_constant_ranges: &[],
//         });

//         let shader = device.create_shader_module(ShaderModuleDescriptor {
//             label: Some("text_shader"),
//             source: wgpu::ShaderSource::Wgsl(include_str!("text.wgsl").into()),
//         });

//         let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
//             label: Some("text_pipeline"),
//             layout: Some(&pipeline_layout),
//             vertex: VertexState {
//                 module: &shader,
//                 entry_point: "vs_main",
//                 buffers: &[Vertex::desc()],
//             },
//             fragment: Some(FragmentState {
//                 module: &shader,
//                 entry_point: "fs_main",
//                 targets: &[Some(ColorTargetState {
//                     format: config.format,
//                     blend: Some(BlendState::ALPHA_BLENDING),
//                     write_mask: ColorWrites::ALL,
//                 })],
//             }),
//             primitive: PrimitiveState {
//                 topology: PrimitiveTopology::TriangleList,
//                 strip_index_format: None,
//                 front_face: wgpu::FrontFace::Ccw,
//                 cull_mode: Some(Face::Back),
//                 polygon_mode: PolygonMode::Fill,
//                 unclipped_depth: false,
//                 conservative: false,
//             },
//             depth_stencil: None,
//             multisample: MultisampleState {
//                 count: 1,
//                 mask: !0,
//                 alpha_to_coverage_enabled: false,
//             },
//             multiview: None,
//         });

//         let sampler = device.create_sampler(&SamplerDescriptor {
//             address_mode_u: AddressMode::ClampToEdge,
//             address_mode_v: AddressMode::ClampToEdge,
//             address_mode_w: AddressMode::ClampToEdge,
//             mag_filter: FilterMode::Linear,
//             min_filter: FilterMode::Nearest,
//             ..Default::default()
//         });

//         let image = render_to_texture();

//         let texture = device.create_texture(&TextureDescriptor {
//             label: Some("texture"),
//             size: Extent3d {
//                 width: image.width(),
//                 height: image.height(),
//                 depth_or_array_layers: 1,
//             },
//             mip_level_count: 1,
//             sample_count: 1,
//             dimension: TextureDimension::D2,
//             format: TextureFormat::Rgba8UnormSrgb,
//             usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
//             view_formats: &[],
//         });

//         queue.write_texture(
//             ImageCopyTexture {
//                 texture: &texture,
//                 mip_level: 0,
//                 origin: Origin3d::ZERO,
//                 aspect: TextureAspect::All,
//             },
//             &image,
//             ImageDataLayout {
//                 offset: 0,
//                 bytes_per_row: Some(4 * image.width()),
//                 rows_per_image: Some(image.height()),
//             },
//             Extent3d {
//                 width: image.width(),
//                 height: image.height(),
//                 depth_or_array_layers: 1,
//             },
//         );

//         let start = super::layout::remap(Vec2::new(0.0, 0.0), size);
//         let end =
//             super::layout::remap(Vec2::new(image.width() as f32, image.height() as f32), size);

//         let vertices = [
//             Vertex {
//                 position: [start.x, start.y, 0.0],
//                 texture: [0.0, 0.0],
//             },
//             Vertex {
//                 position: [start.x, end.y, 0.0],
//                 texture: [0.0, 1.0],
//             },
//             Vertex {
//                 position: [end.x, end.y, 0.0],
//                 texture: [1.0, 1.0],
//             },
//             Vertex {
//                 position: [end.x, start.y, 0.0],
//                 texture: [1.0, 0.0],
//             },
//         ];
//         let indicies = [0, 1, 2, 3, 0, 2];

//         let vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
//             label: Some("text_vertex_buffer"),
//             contents: bytemuck::cast_slice(&vertices),
//             usage: BufferUsages::VERTEX,
//         });

//         let index_buffer = device.create_buffer_init(&BufferInitDescriptor {
//             label: Some("text_index_buffer"),
//             contents: bytemuck::cast_slice(&indicies),
//             usage: BufferUsages::INDEX,
//         });

//         Self {
//             pipeline,
//             layout: bind_group_layout,
//             sampler,
//             buffer: None,
//             vertex_buffer,
//             index_buffer,
//             size: Vec2::splat(0.0),
//         }
//     }
// }

impl Text {
    // pub fn render<'a>(
    //     &self,
    //     state: &'a State,
    //     bind_groups: &'a mut Vec<BindGroup>,
    //     render_pass: &mut RenderPass<'a>,
    // ) {
    //     let pipeline = &state.text_pipeline;

    //     let texture_view = pipeline
    //         .buffer
    //         .as_ref()
    //         .unwrap()
    //         .create_view(&TextureViewDescriptor::default());

    //     bind_groups.push(state.device.create_bind_group(&BindGroupDescriptor {
    //         layout: &pipeline.layout,
    //         entries: &[
    //             BindGroupEntry {
    //                 binding: 0,
    //                 resource: BindingResource::TextureView(&texture_view),
    //             },
    //             BindGroupEntry {
    //                 binding: 1,
    //                 resource: BindingResource::Sampler(&pipeline.sampler),
    //             },
    //         ],
    //         label: Some("texture_bind_group"),
    //     }));

    //     render_pass.set_pipeline(&pipeline.pipeline);
    //     render_pass.set_bind_group(0, &bind_groups.last().unwrap(), &[]);

    //     render_pass.set_vertex_buffer(0, pipeline.vertex_buffer.slice(..));
    //     render_pass.set_index_buffer(pipeline.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

    //     render_pass.draw_indexed(0..6, 0, 0..1);
    // }
}

// impl Widget for Text {
// fn draw(&self, ctx: &mut DrawContext) {
//     let font = FontRef::try_from_slice(include_bytes!("/usr/share/fonts/droid/DroidSans.ttf"))
//         .unwrap();

//     let scaled_font = font.as_scaled(PxScale::from(24.0));

//     let mut glyphs = Vec::new();
//     layout_glyphs(scaled_font, point(20.0, 20.0), "Hello World!", &mut glyphs);

//     let height = scaled_font.height().ceil() as u32;
//     let width = {
//         let min_x = glyphs.first().unwrap().position.x;
//         let last_glyph = glyphs.last().unwrap();
//         let max_x = last_glyph.position.x + scaled_font.h_advance(last_glyph.id);
//         // dbg!(max_x);
//         // (max_x - min_x).ceil() as u32
//         max_x.ceil() as u32
//     };

//     // let mut image: ImageBuffer<Luma<u8>, Vec<u8>> = ImageBuffer::new(width + 40, height + 40);
//     let mut image = RgbaImage::new(width + 40, height + 40);

//     // for pixel in image.pixels_mut() {
//     //     *pixel = Rgba([0, 0, 0, 255]);
//     // }

//     for glyph in glyphs {
//         if let Some(outlined_glyph) = scaled_font.outline_glyph(glyph) {
//             let bounds = outlined_glyph.px_bounds();

//             outlined_glyph.draw(|x, y, alpha| {
//                 let pixel = (alpha * 255.0) as u8;

//                 image.put_pixel(
//                     bounds.min.x as u32 + x,
//                     bounds.min.y as u32 + y,
//                     // Luma([pixel]),
//                     Rgba([pixel, 0, 0, pixel]),
//                 );
//             });
//         }
//     }

//     let texture = ctx.device().create_texture(&TextureDescriptor {
//         label: Some("texture"),
//         size: Extent3d {
//             width: image.width(),
//             height: image.height(),
//             depth_or_array_layers: 1,
//         },
//         mip_level_count: 1,
//         sample_count: 1,
//         dimension: TextureDimension::D2,
//         format: TextureFormat::Rgba8UnormSrgb,
//         usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
//         view_formats: &[],
//     });

//     ctx.queue().write_texture(
//         ImageCopyTexture {
//             texture: &texture,
//             mip_level: 0,
//             origin: Origin3d::ZERO,
//             aspect: TextureAspect::All,
//         },
//         &image,
//         ImageDataLayout {
//             offset: 0,
//             bytes_per_row: Some(4 * image.width()),
//             rows_per_image: Some(image.height()),
//         },
//         Extent3d {
//             width: image.width(),
//             height: image.height(),
//             depth_or_array_layers: 1,
//         },
//     );

//     let text_pipeline = &mut ctx.text_pipeline;
//     text_pipeline.buffer = Some(texture);

//     text_pipeline.size = Vec2::new(width as f32 + 40.0, height as f32 + 40.0);

//     // let texture_view = texture.create_view(&TextureViewDescriptor::default());

//     // let bind_group = ctx.device().create_bind_group(&BindGroupDescriptor {
//     //     layout: &text_pipeline.layout,
//     //     entries: &[
//     //         BindGroupEntry {
//     //             binding: 0,
//     //             resource: BindingResource::TextureView(&texture_view),
//     //         },
//     //         BindGroupEntry {
//     //             binding: 1,
//     //             resource: BindingResource::Sampler(&text_pipeline.sampler),
//     //         },
//     //     ],
//     //     label: Some("texture_bind_group"),
//     // });

//     // ctx.bind_groups.push(bind_group);

//     image.save("test.png").unwrap();
//     }
// }

fn render_to_texture(text: &str) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let font =
        FontRef::try_from_slice(include_bytes!("/usr/share/fonts/droid/DroidSans.ttf")).unwrap();

    let scaled_font = font.as_scaled(PxScale::from(60.0));

    let mut glyphs = Vec::new();
    let (num_lines, max_width) =
        layout_glyphs(scaled_font, point(0.0, 0.0), text, 1000.0, &mut glyphs);

    let height = scaled_font.height().ceil() as u32;

    let mut image = RgbaImage::new(max_width.ceil() as u32, num_lines * height);

    // for pixel in image.pixels_mut() {
    //     *pixel = Rgba([0, 0, 0, 255]);
    // }

    for glyph in glyphs {
        if let Some(outlined_glyph) = scaled_font.outline_glyph(glyph) {
            let bounds = outlined_glyph.px_bounds();

            outlined_glyph.draw(|x, y, alpha| {
                let pixel = (alpha * 255.0) as u8;

                image.put_pixel(
                    bounds.min.x as u32 + x,
                    bounds.min.y as u32 + y,
                    // Luma([pixel]),
                    Rgba([pixel, 0, 0, pixel]),
                );
            });
        }
    }

    image
}

fn layout_glyphs<SF: ScaleFont<F>, F: Font>(
    font: SF,
    position: Point,
    text: &str,
    max_width: f32,
    target: &mut Vec<Glyph>,
) -> (u32, f32) {
    let mut num_lines = 1;

    // The width of the widest line.
    let mut width = 0.0;

    let v_advance = font.height() + font.line_gap();
    let mut caret = position + point(0.0, font.ascent());

    let mut last_glyph: Option<Glyph> = None;

    for ch in text.chars() {
        if ch.is_control() {
            if ch == '\n' {
                if caret.x > width {
                    width = caret.x;
                }

                caret = point(position.x, caret.y + v_advance);
                num_lines += 1;
            }

            continue;
        }

        let mut glyph = font.scaled_glyph(ch);
        if let Some(prev) = last_glyph.take() {
            caret.x += font.kern(prev.id, glyph.id);
        }

        glyph.position = caret;

        last_glyph = Some(glyph.clone());
        caret.x += font.h_advance(glyph.id);

        if !ch.is_whitespace() && caret.x > position.x + max_width {
            if caret.x > width {
                width = caret.x;
            }

            caret = point(caret.x, position.y);
            glyph.position = caret;
            last_glyph = None;
            num_lines += 1;
        }

        target.push(glyph);
    }

    if caret.x > width {
        width = caret.x;
    }

    (num_lines, width)
}

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
struct Vertex {
    position: [f32; 3],
    texture: [f32; 2],
}

impl Vertex {
    fn desc<'a>() -> VertexBufferLayout<'a> {
        VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as BufferAddress,
            step_mode: VertexStepMode::Vertex,
            attributes: &[
                VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: VertexFormat::Float32x3,
                },
                VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as BufferAddress,
                    shader_location: 1,
                    format: VertexFormat::Float32x2,
                },
            ],
        }
    }
}
