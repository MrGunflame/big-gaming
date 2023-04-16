use std::ffi::OsStr;
use std::fs::File;
use std::io::Read;

use crate::host::native::NativeScript;
use crate::host::wasm::WasmScript;
use crate::ScriptServer;

pub enum Script {
    Native(NativeScript),
    Wasm(WasmScript),
}

impl Script {
    pub fn load<P: AsRef<OsStr>>(server: &ScriptServer, path: P) -> Self {
        let mut file = File::open(path.as_ref()).unwrap();

        // Read ELF header
        let mut buf = [0; 4];
        file.read_exact(&mut buf).unwrap();

        // TODO: Non-ELF targets
        match buf {
            MAGIC_ELF => Self::Native(NativeScript::new(path).unwrap()),
            MAGIC_WASM => Self::Wasm(WasmScript::new(path, &server.engine)),
            _ => panic!("unknown file type"),
        }
    }

    pub fn run(&self) {
        match self {
            Self::Native(sc) => sc.run(),
            Self::Wasm(sc) => sc.run(),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ScriptKind {
    Native,
}

const MAGIC_ELF: [u8; 4] = [0x7f, 0x45, 0x4c, 0x46];
const MAGIC_WASM: [u8; 4] = [0x00, 0x61, 0x73, 0x6d];
