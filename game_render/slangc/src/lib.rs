use std::collections::HashSet;
use std::env::temp_dir;
use std::io;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use thiserror::Error;

#[cfg(target_os = "linux")]
const SLANGC_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../external/slang/linux-x86_64/bin/slangc"
);
#[cfg(target_os = "windows")]
const SLANGC_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../external/slang/windows-x86_64/bin/slangc"
);
#[cfg(all(not(target_os = "linux"), not(target_os = "windows")))]
compile_error!("no slangc available on this OS");

#[derive(Clone, Debug, Error)]
#[error("{0}")]
pub struct Error(String);

#[derive(Copy, Clone, Debug)]
#[repr(u8)]
pub enum OptLevel {
    None = 0,
    Default = 1,
    High = 2,
    Max = 3,
}

pub fn compile<P>(input: P, opt_level: OptLevel) -> Result<Vec<u8>, Error>
where
    P: AsRef<Path>,
{
    let name = format!("{}{}", std::process::id(), rand::random::<u64>());
    let mut path = temp_dir().join(name);
    path.set_extension("spv");
    let path = RemoveOnDrop(path);

    let mut args = Vec::new();
    // SPIR-V 1.6
    args.extend(["-target", "spirv"]);
    args.extend(["-profile", "spirv_1_6"]);

    args.push("-fvk-use-gl-layout");

    // Flip Y of glPosition, like naga.
    args.push("-fvk-invert-y");

    // Use column major for matrices.
    // This is the same as GLSL and glam.
    args.push("-matrix-layout-column-major");

    let opt_level = format!("-O{}", opt_level as u8);
    args.push(&opt_level);

    args.extend(["-o", path.to_str().unwrap()]);
    args.extend([input.as_ref().to_str().unwrap()]);

    tracing::info!("{} {}", SLANGC_PATH, args.join(" "));

    match Command::new(SLANGC_PATH)
        .args(&args)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()
    {
        Ok(output) if !output.status.success() => {
            return Err(Error(format!("bad status code: {}", output.status)));
        }
        Ok(_) => (),
        Err(err) => return Err(Error(err.to_string())),
    };

    let bytes = std::fs::read(&*path).map_err(|err| Error(err.to_string()))?;
    Ok(bytes)
}

#[derive(Debug)]
struct RemoveOnDrop(PathBuf);

impl Deref for RemoveOnDrop {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Drop for RemoveOnDrop {
    fn drop(&mut self) {
        std::fs::remove_file(&self.0).ok();
    }
}

pub fn load_imported_files(path: &Path) -> io::Result<Vec<PathBuf>> {
    let mut queue = vec![PathBuf::from(path)];
    let mut visited = HashSet::new();

    while let Some(p) = queue.pop() {
        let contents = std::fs::read_to_string(&p)?;
        visited.insert(p);

        for line in contents.lines() {
            let Some(mut line) = line.strip_prefix("import ") else {
                continue;
            };

            if let Some(s) = line.strip_suffix(";") {
                line = s;
            }

            let mut path = PathBuf::from(path);
            path.set_file_name(line.trim());
            path.set_extension("slang");
            queue.push(path);
        }
    }

    Ok(visited.into_iter().collect())
}
