use std::ffi::OsStr;
use std::io;
use std::num::NonZeroU32;
use std::path::PathBuf;

use hashbrown::HashMap;
use naga::back::spv::{self, PipelineOptions};
use naga::front::wgsl;
use naga::valid::{Capabilities, GlobalUse, ModuleInfo, ValidationFlags, Validator};
use naga::{AddressSpace, ArraySize, Module, TypeInner};
use thiserror::Error;

use crate::backend::{DescriptorType, ShaderStage};

use super::{
    BindingInfo, BindingLocation, Options, ShaderAccess, ShaderBinding, ShaderSource, ShaderSources,
};

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Parse(wgsl::ParseError),
    #[error(transparent)]
    Validation(naga::WithSpan<naga::valid::ValidationError>),
}

#[derive(Clone, Debug)]
pub struct Shader {
    module: Module,
    info: ModuleInfo,
}

impl Shader {
    pub fn from_wgsl(s: &str) -> Result<Self, Error> {
        let module = wgsl::parse_str(s).map_err(Error::Parse)?;

        let mut validator = Validator::new(ValidationFlags::default(), Capabilities::all());
        let info = validator
            .validate_no_overrides(&module)
            .map_err(Error::Validation)?;

        Ok(Self { module, info })
    }

    // TODO: Create this when self is created.
    pub fn bindings(&self) -> Vec<ShaderBinding> {
        self.shader_bindings(None)
    }

    pub fn instantiate(&self, options: &Options<'_>) -> ShaderInstance<'_> {
        let bindings = self.shader_bindings(Some(options.entry_point));

        ShaderInstance {
            shader: self,
            bindings,
            entry_point: options.entry_point.to_string(),
            stage: options.stage,
            binding_map: options.bindings.clone(),
        }
    }

    fn shader_bindings(&self, entry_point: Option<&str>) -> Vec<ShaderBinding> {
        let mut bindings = Vec::new();

        let functions = match entry_point {
            Some(name) => {
                let index = self
                    .module
                    .entry_points
                    .iter()
                    .position(|f| f.name == name)
                    .unwrap();
                vec![self.info.get_entry_point(index)]
            }
            None => (0..self.module.entry_points.len())
                .map(|index| self.info.get_entry_point(index))
                .collect(),
        };

        for (handle, var) in self.module.global_variables.iter() {
            let Some(binding) = &var.binding else {
                continue;
            };

            let (kind, count) = match &self.module.types[var.ty].inner {
                TypeInner::BindingArray { base, size } => {
                    let kind = match self.module.types[*base].inner {
                        TypeInner::Image {
                            dim: _,
                            arrayed: _,
                            class: _,
                        } => DescriptorType::Texture,
                        _ => todo!(),
                    };

                    let count = match size {
                        ArraySize::Constant(size) => Some(*size),
                        ArraySize::Dynamic => None,
                    };

                    (kind, count)
                }
                TypeInner::Image {
                    dim: _,
                    arrayed: _,
                    class: _,
                } => (DescriptorType::Texture, Some(NonZeroU32::MIN)),
                TypeInner::Sampler { comparison: _ } => {
                    (DescriptorType::Sampler, Some(NonZeroU32::MIN))
                }
                TypeInner::Scalar(_)
                | TypeInner::Vector { size: _, scalar: _ }
                | TypeInner::Matrix {
                    columns: _,
                    rows: _,
                    scalar: _,
                }
                | TypeInner::Struct {
                    members: _,
                    span: _,
                }
                | TypeInner::Array {
                    base: _,
                    size: _,
                    stride: _,
                } => {
                    let kind = match var.space {
                        AddressSpace::Uniform => DescriptorType::Uniform,
                        AddressSpace::Storage { access: _ } => DescriptorType::Storage,
                        AddressSpace::PushConstant => continue,
                        _ => todo!(),
                    };

                    (kind, Some(NonZeroU32::MIN))
                }
                _ => todo!(),
            };

            let access: ShaderAccess = functions
                .iter()
                .flat_map(|function| {
                    function[handle].iter().filter_map(|flags| match flags {
                        GlobalUse::READ => Some(ShaderAccess::READ),
                        GlobalUse::WRITE => Some(ShaderAccess::WRITE),
                        _ => None,
                    })
                })
                .collect();

            if access.is_empty() {
                continue;
            }

            bindings.push(ShaderBinding {
                group: binding.group,
                binding: binding.binding,
                kind,
                access,
                count,
            });
        }

        bindings
    }
}

#[derive(Clone, Debug)]
pub struct ShaderInstance<'a> {
    shader: &'a Shader,
    bindings: Vec<ShaderBinding>,
    entry_point: String,
    stage: ShaderStage,
    binding_map: HashMap<BindingLocation, BindingInfo>,
}

impl<'a> ShaderInstance<'a> {
    pub fn bindings(&self) -> &[ShaderBinding] {
        &self.bindings
    }

    pub fn to_spirv(&self) -> Vec<u32> {
        let mut options = spv::Options::default();
        for (location, info) in &self.binding_map {
            options.binding_map.insert(
                naga::ResourceBinding {
                    group: location.group,
                    binding: location.binding,
                },
                spv::BindingInfo {
                    binding_array_size: Some(info.count.get()),
                },
            );
        }

        let shader_stage = match self.stage {
            ShaderStage::Vertex => naga::ShaderStage::Vertex,
            ShaderStage::Fragment => naga::ShaderStage::Fragment,
            ShaderStage::Task | ShaderStage::Mesh => {
                panic!("unsupported WGSL stage: {:?}", self.stage)
            }
        };

        spv::write_vec(
            &self.shader.module,
            &self.shader.info,
            &options,
            Some(&PipelineOptions {
                shader_stage,
                entry_point: self.entry_point.clone(),
            }),
        )
        .unwrap()
    }
}

pub fn load_files(root: ShaderSource) -> io::Result<ShaderSources> {
    let root_dir = match &root {
        ShaderSource::File(path) => path.parent().map(|s| s.as_os_str()).unwrap_or_default(),
        _ => <&OsStr>::default(),
    };

    let mut files = Vec::new();
    let mut sources = Vec::new();
    let mut queue = vec![root.clone()];

    while let Some(src) = queue.pop() {
        let data = src.load()?;
        sources.push(src);

        for line in data.lines() {
            if line.starts_with("//") {
                continue;
            }

            let Some(path) = line.strip_prefix("#include") else {
                continue;
            };

            let mut file_path = PathBuf::from(root_dir);
            file_path.push(PathBuf::from(path.trim()));
            if !path.is_empty() {
                queue.push(ShaderSource::File(file_path));
            }
        }

        files.push(data);
    }

    Ok(ShaderSources {
        sources,
        data: files,
    })
}
