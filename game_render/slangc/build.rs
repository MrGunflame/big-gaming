use std::process::Command;

const SLANG_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/vendor/slang");
const SLANGC: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/vendor/slang/build/Release/bin/slangc"
);

fn main() {
    println!("cargo::rerun-if-changed=vendor/slang");

    // Don't build if slangc already exists.
    // This significantly reduces build times after the first build.
    if std::fs::exists(SLANGC).unwrap_or_default() {
        return;
    }

    spawn("cmake --preset default");
    spawn("cmake --build --preset release");
}

fn spawn(program: &str) {
    let mut parts = program.split(" ");
    let name = parts.next().unwrap();
    let args = parts.collect::<Vec<_>>();

    let status = Command::new(name)
        .current_dir(SLANG_PATH)
        .args(&args)
        .status()
        .expect("failed to build slang");

    if !status.success() {
        panic!(
            "failed to build slangc, \"{}\" exited with error status",
            program
        );
    }
}
