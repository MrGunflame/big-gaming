use slangc::OptLevel;

#[test]
fn compile_test() {
    let input = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/test.slang");
    slangc::compile(input, OptLevel::None).unwrap();
}
