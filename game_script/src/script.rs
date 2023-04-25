use std::ffi::OsStr;
use std::fs::File;
use std::io::Read;

use crate::host::wasm::WasmScript;
use crate::ScriptServer;

pub enum Script {
    Wasm(WasmScript),
}

impl Script {
    pub fn load<P: AsRef<OsStr>>(
        server: &ScriptServer,
        path: P,
        // FIXME: Replace with strongly-typed error type.
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let mut file = File::open(path.as_ref())?;

        // Read header
        let mut buf = [0; 4];
        file.read_exact(&mut buf)?;

        match buf {
            MAGIC_WASM => Ok(Self::Wasm(WasmScript::new(path, &server.engine)?)),
            _ => panic!("unknown file type"),
        }
    }

    pub fn run(&self) {
        match self {
            Self::Wasm(sc) => sc.run(),
        }
    }
}

const MAGIC_WASM: [u8; 4] = [0x00, 0x61, 0x73, 0x6d];
