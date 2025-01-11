use naga::back::spv::PipelineOptions;
use naga::front::wgsl;
use naga::valid::{Capabilities, ValidationFlags, Validator};
use naga::ShaderStage;

pub fn wgsl_to_spirv(s: &str) -> Vec<u32> {
    let module = wgsl::parse_str(s).unwrap();

    let mut validator = Validator::new(ValidationFlags::default(), Capabilities::default());
    let info = validator.validate(&module).unwrap();

    naga::back::spv::write_vec(&module, &info, &Default::default(), None).unwrap()
}
