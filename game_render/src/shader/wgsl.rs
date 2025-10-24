use naga::back::spv;
use naga::front::wgsl;
use naga::valid::{Capabilities, ShaderStages, SubgroupOperationSet, ValidationFlags, Validator};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Parse(wgsl::ParseError),
    #[error(transparent)]
    Validation(naga::WithSpan<naga::valid::ValidationError>),
    #[error(transparent)]
    Codegen(spv::Error),
}

pub fn compile(input: &str) -> Result<Vec<u8>, Error> {
    let module = wgsl::parse_str(input).map_err(Error::Parse)?;

    let info = Validator::new(ValidationFlags::all(), Capabilities::all())
        .subgroup_stages(ShaderStages::all())
        .subgroup_operations(SubgroupOperationSet::all())
        .validate(&module)
        .map_err(Error::Validation)?;

    let spv =
        spv::write_vec(&module, &info, &spv::Options::default(), None).map_err(Error::Codegen)?;
    Ok(spv.into_iter().flat_map(|v| v.to_ne_bytes()).collect())
}
