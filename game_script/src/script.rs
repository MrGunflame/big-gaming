use std::ffi::OsStr;
use std::fs::File;
use std::io::Read;

use crate::host::native::NativeScript;

#[derive(Debug)]
pub enum Script {
    Native(NativeScript),
}

impl Script {
    pub fn load<P: AsRef<OsStr>>(path: P) -> Self {
        let mut file = File::open(path.as_ref()).unwrap();

        // Read ELF header
        let mut buf = [0; 4];
        file.read_exact(&mut buf).unwrap();

        // TODO: Non-ELF targets
        match buf {
            MAGIC_ELF => Self::Native(NativeScript::new(path).unwrap()),
            _ => panic!("unknown file type"),
        }
    }

    pub fn run(&self) {
        match self {
            Self::Native(sc) => sc.run(),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ScriptKind {
    Native,
}

const MAGIC_ELF: [u8; 4] = [0x7f, 0x45, 0x4c, 0x46];
