use std::env;
use std::path::PathBuf;
use std::process::Command;

const SOURCE_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/vendor/slang");
const BINARY_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/vendor/slang/build/Release/bin"
);

fn main() {
    println!("cargo::rerun-if-changed=vendor/slang");

    // FIXME: We should directly output the cmake data into OUT_DIR instead of
    // doing the copy thing.
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR is not set"));

    let mut slangc_dst = out_dir.join("slangc");
    if cfg!(target_family = "windows") {
        slangc_dst.set_extension("exe");
    }

    // Load the current git version and compare it against the currently compiled
    // binary. This ensures we always recompile if the upstream version changes.
    let source_version = get_git_rev();
    let binary_version = std::fs::read_to_string(out_dir.join("version")).unwrap_or_default();

    // Don't build if slangc already exists.
    // This significantly reduces build times after the first build.
    if source_version == binary_version && std::fs::exists(&slangc_dst).unwrap_or_default() {
        return;
    }

    let options = [
        "-DSLANG_LIB_TYPE=STATIC",
        // Don't build tests and examples and their dependencies.
        "-DSLANG_ENABLE_TESTS=0",
        "-DSLANG_ENABLE_SLANG_RHI=0",
        "-DSLANG_ENABLE_EXAMPLES=0",
        "-DSLANG_ENABLE_GFX=0",
        // We not emit DXIL.
        "-DSLANG_ENABLE_DXIL=0",
        // We only need slangc.
        "-DSLANG_ENABLE_SLANGD=0",
        "-DSLANG_ENABLE_SLANGI=0",
        "-DSLANG_ENABLE_SLANGRT=0",
    ];

    // Disable tests, since they come with rendering dependencies.
    spawn(&format!("cmake --preset default {}", options.join(" ")));
    spawn("cmake --build --preset release");

    let mut slangc_src = PathBuf::from(BINARY_PATH).join("slangc");
    if cfg!(target_family = "windows") {
        slangc_src.set_extension("exe");
    }

    std::fs::write(out_dir.join("version"), source_version.as_bytes())
        .expect("failed to write version");

    // Move the slangc binary to OUT_DIR, then remove build data
    // from the slang path.
    std::fs::rename(slangc_src, slangc_dst).expect("failed to move slangc file");
    std::fs::remove_dir_all(PathBuf::from(SOURCE_PATH).join("build")).expect("failed to cleanup");
}

fn spawn(program: &str) {
    let mut parts = program.split(" ");
    let name = parts.next().unwrap();
    let args = parts.collect::<Vec<_>>();

    let status = Command::new(name)
        .current_dir(SOURCE_PATH)
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

fn get_git_rev() -> String {
    let output = Command::new("git")
        .current_dir(SOURCE_PATH)
        .args(&["rev-parse", "HEAD"])
        .output()
        .expect("failed to fetch git revision");
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}
