use naga::back::spv::PipelineOptions;
use naga::front::wgsl;
use naga::valid::{Capabilities, ValidationFlags, Validator};
use naga::{AddressSpace, ResourceBinding, ShaderStage, TypeInner};

use super::DescriptorType;

#[derive(Clone, Debug)]
pub struct ShaderInfo {
    pub bindings: Vec<ShaderBinding>,
}

pub fn wgsl_to_spirv(s: &str) -> (Vec<u32>, ShaderInfo) {
    let module = wgsl::parse_str(s).unwrap();

    let mut bindings = Vec::new();
    for (handle, var) in module.global_variables.iter() {
        let Some(binding) = &var.binding else {
            continue;
        };

        let kind = match module.types[var.ty].inner {
            TypeInner::Image {
                dim: _,
                arrayed: _,
                class: _,
            } => DescriptorType::Texture,
            TypeInner::Sampler { comparison: _ } => DescriptorType::Sampler,
            _ => match &var.space {
                AddressSpace::Uniform => DescriptorType::Uniform,
                AddressSpace::Storage { access: _ } => DescriptorType::Storage,
                _ => continue,
            },
        };

        bindings.push(ShaderBinding {
            group: binding.group,
            binding: binding.binding,
            kind,
        });
    }

    let mut validator = Validator::new(ValidationFlags::default(), Capabilities::all());
    let info = validator.validate(&module).unwrap();

    (
        naga::back::spv::write_vec(&module, &info, &Default::default(), None).unwrap(),
        ShaderInfo { bindings },
    )
}

#[derive(Copy, Clone, Debug)]
pub struct ShaderBinding {
    pub group: u32,
    pub binding: u32,
    pub kind: DescriptorType,
}
