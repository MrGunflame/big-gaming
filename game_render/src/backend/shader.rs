use naga::back::spv::PipelineOptions;
use naga::front::glsl::{Frontend, Options};
use naga::valid::{Capabilities, ValidationFlags, Validator};
use naga::ShaderStage;

pub fn glsl_to_spirv(s: &str, stage: ShaderStage) -> Vec<u32> {
    let module = Frontend::default()
        .parse(
            &Options {
                stage,
                defines: Default::default(),
            },
            s,
        )
        .unwrap();

    let mut validator = Validator::new(ValidationFlags::default(), Capabilities::default());
    let info = validator.validate(&module).unwrap();

    naga::back::spv::write_vec(
        &module,
        &info,
        &Default::default(),
        Some(&PipelineOptions {
            shader_stage: stage,
            entry_point: "main".to_owned(),
        }),
    )
    .unwrap()
}
