use std::collections::HashMap;
use std::num::NonZeroU32;

use bitflags::bitflags;
use naga::back::spv::{self, PipelineOptions};
use naga::front::wgsl;
use naga::valid::{Capabilities, GlobalUse, ModuleInfo, ValidationFlags, Validator};
use naga::{AddressSpace, ArraySize, Module, TypeInner};

use super::{DescriptorType, ShaderStage};

#[derive(Clone, Debug)]
pub struct Shader {
    module: Module,
    info: ModuleInfo,
}

impl Shader {
    pub fn from_wgsl(s: &str) -> Self {
        let module = wgsl::parse_str(s).unwrap();

        let mut validator = Validator::new(ValidationFlags::default(), Capabilities::all());
        let info = validator.validate_no_overrides(&module).unwrap();

        Self { module, info }
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

            let access = functions
                .iter()
                .flat_map(|function| {
                    function[handle].iter().filter_map(|flags| match flags {
                        GlobalUse::READ => Some(ShaderAccess::READ),
                        GlobalUse::WRITE => Some(ShaderAccess::WRITE),
                        _ => None,
                    })
                })
                .collect();

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

#[derive(Copy, Clone, Debug)]
pub struct ShaderBinding {
    pub group: u32,
    pub binding: u32,
    pub kind: DescriptorType,
    pub access: ShaderAccess,
    /// If the binding point is an binding array this will be greater than 1.
    ///
    /// This is always 1 for non-array types.
    ///
    /// `None` indicates that the count is still undefined and needs to specialized on
    /// instantiation.
    pub count: Option<NonZeroU32>,
}

impl ShaderBinding {
    pub fn location(&self) -> BindingLocation {
        BindingLocation {
            group: self.group,
            binding: self.binding,
        }
    }
}

bitflags! {
    #[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
    pub struct ShaderAccess: u8 {
        /// The resource will be read from.
        const READ = 1 << 0;
        /// The resource will be written to.
        const WRITE = 1 << 1;
    }
}

#[derive(Clone, Debug)]
pub struct Options<'a> {
    pub entry_point: &'a str,
    pub stage: ShaderStage,
    pub bindings: HashMap<BindingLocation, BindingInfo>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct BindingLocation {
    pub group: u32,
    pub binding: u32,
}

#[derive(Clone, Debug)]
pub struct BindingInfo {
    pub count: NonZeroU32,
}
