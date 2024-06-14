use std::borrow::Cow;
use std::fs::File;
use std::io::Read;
use std::num::NonZeroU32;

use naga::front::wgsl::ParseError;
use naga::valid::{Capabilities, ValidationFlags, Validator};
use wgpu::hal::{DebugSource, Device as _, NagaShader};
use wgpu::{
    ColorTargetState, DepthStencilState, Device, MultisampleState, PipelineLayout, PrimitiveState,
    RenderPipeline, ShaderModule, ShaderModuleDescriptor, ShaderSource, VertexBufferLayout,
};

#[derive(Debug)]
pub struct Pipeline<T>
where
    T: PipelineObject,
{
    pipeline: Option<T>,
    descriptor: T::Descriptor,
}

impl<T> Pipeline<T>
where
    T: PipelineObject,
{
    pub fn new(device: &Device, descriptor: T::Descriptor) -> Self {
        let pipeline = T::build(&descriptor, device);

        Self {
            pipeline: Some(pipeline),
            descriptor,
        }
    }

    pub fn get(&self) -> &T {
        self.pipeline.as_ref().unwrap()
    }
}

pub trait PipelineObject {
    type Descriptor;

    fn build(descriptor: &Self::Descriptor, device: &Device) -> Self;
}

impl PipelineObject for RenderPipeline {
    type Descriptor = RenderPipelineDescriptor<'static>;

    fn build(descriptor: &Self::Descriptor, device: &Device) -> Self {
        let vertex_shader = descriptor.vertex.module.load(device).unwrap();

        let fragment_shader = descriptor
            .fragment
            .as_ref()
            .map(|state| state.module.load(device).unwrap());

        let fragment = if let Some(fragment) = &descriptor.fragment {
            Some(wgpu::FragmentState {
                module: fragment_shader.as_ref().unwrap(),
                entry_point: fragment.entry_point,
                targets: fragment.targets,
            })
        } else {
            None
        };

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&descriptor.layout),
            vertex: wgpu::VertexState {
                module: &vertex_shader,
                entry_point: descriptor.vertex.entry_point,
                buffers: descriptor.vertex.buffers,
            },
            fragment,
            primitive: descriptor.primitive,
            depth_stencil: descriptor.depth_stencil.clone(),
            multisample: descriptor.multisample,
            multiview: descriptor.multiview,
        })
    }
}

#[derive(Debug)]
pub struct RenderPipelineDescriptor<'a> {
    pub layout: PipelineLayout,
    pub vertex: VertexState<'a>,
    pub fragment: Option<FragmentState<'a>>,
    pub primitive: PrimitiveState,
    pub depth_stencil: Option<DepthStencilState>,
    pub multisample: MultisampleState,
    pub multiview: Option<NonZeroU32>,
}

#[derive(Debug)]
pub struct VertexState<'a> {
    pub module: Shader,
    pub entry_point: &'a str,
    pub buffers: &'a [VertexBufferLayout<'a>],
}

#[derive(Debug)]
pub struct FragmentState<'a> {
    pub module: Shader,
    pub entry_point: &'a str,
    pub targets: &'a [Option<ColorTargetState>],
}

#[derive(Debug)]
pub struct Shader {
    path: &'static str,
}

impl Shader {
    pub fn from_file(path: &'static str) -> Self {
        Self { path }
    }

    fn load(&self, device: &Device) -> Result<ShaderModule, ShaderError> {
        let mut file = File::open(self.path).map_err(ShaderError::Io)?;

        let mut buf = String::new();
        file.read_to_string(&mut buf).map_err(ShaderError::Io)?;

        let module = naga::front::wgsl::parse_str(&buf).map_err(ShaderError::Naga)?;

        let mut validator = Validator::new(ValidationFlags::all(), Capabilities::all());
        let info = validator.validate(&module).unwrap();

        if let Err(err) = unsafe {
            device
                .as_hal::<wgpu_core::api::Vulkan, _, _>(|hal| {
                    let hal: &wgpu::hal::vulkan::Device = hal.as_ref().unwrap();

                    match hal.create_shader_module(
                        &wgpu::hal::ShaderModuleDescriptor {
                            label: None,
                            runtime_checks: true,
                        },
                        wgpu::hal::ShaderInput::Naga(NagaShader {
                            module: Cow::Owned(module),
                            info,
                            debug_source: Some(DebugSource {
                                file_name: self.path.into(),
                                source_code: Cow::Owned(buf.clone()),
                            }),
                        }),
                    ) {
                        Ok(shader) => {
                            hal.destroy_shader_module(shader);
                            Ok(())
                        }
                        Err(err) => Err(ShaderError::Shader(err)),
                    }
                })
                .unwrap()
        } {
            return Err(err);
        }

        Ok(device.create_shader_module(ShaderModuleDescriptor {
            label: None,
            source: ShaderSource::Wgsl(buf.into()),
        }))
    }
}

#[derive(Debug)]
enum ShaderError {
    Io(std::io::Error),
    Naga(ParseError),
    Shader(wgpu::hal::ShaderError),
}
