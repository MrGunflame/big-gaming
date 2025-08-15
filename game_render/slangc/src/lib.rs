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

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Io(io::Error),
    #[error("{0}")]
    Other(String),
}

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
    let input = input.as_ref();

    let files = load_imported_files(input)?;
    let mut search_paths = Vec::new();
    for dir in &files {
        let dir = dir.parent().unwrap().to_str().unwrap();
        if !search_paths.contains(&dir) {
            search_paths.push("-I");
            search_paths.push(dir);
        }
    }

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
    args.extend(&search_paths);
    args.push(input.to_str().unwrap());

    tracing::info!("{} {}", SLANGC_PATH, args.join(" "));

    match Command::new(SLANGC_PATH)
        .args(&args)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()
    {
        Ok(output) if !output.status.success() => {
            return Err(Error::Other(format!("bad status code: {}", output.status)));
        }
        Ok(_) => (),
        Err(err) => return Err(Error::Other(err.to_string())),
    };

    let bytes = std::fs::read(&*path).map_err(Error::Io)?;
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

pub fn load_imported_files(path: &Path) -> Result<Vec<PathBuf>, Error> {
    let mut queue = vec![PathBuf::from(path)];
    let mut visited = HashSet::new();

    while let Some(mut p) = queue.pop() {
        while !p.try_exists().map_err(Error::Io)? {
            let file_name = p.file_name().unwrap().to_owned();
            let extension = p.extension().unwrap().to_owned();
            // Remove the file name and extension.

            // Remove the parent directory.
            if !p.pop() {
                return Err(Error::Other(format!(
                    "cannot find import: {}",
                    file_name.to_str().unwrap(),
                )));
            }

            // Add the file name and extension back.
            p.set_file_name(file_name);
            p.set_extension(extension);
        }

        let contents = std::fs::read_to_string(&p).map_err(Error::Io)?;
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
