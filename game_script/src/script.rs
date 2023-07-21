use std::fs::File;
use std::io::Read;
use std::path::Path;

use wasmtime::{Engine, ExternType, Module};

use crate::events::Events;

const MAGIC_WASM: [u8; 4] = [0x00, 0x61, 0x73, 0x6d];

pub(crate) struct Script {
    pub module: Module,
    pub events: Events,
}

impl Script {
    pub fn load(path: &Path, engine: &Engine) -> Result<Self, Box<dyn std::error::Error>> {
        let mut file = File::open(path)?;

        let mut buf = Vec::new();
        file.read_to_end(&mut buf)?;

        let module = Module::new(engine, buf)?;

        let mut events = Events::NONE;

        for (name, event) in [("on_action", Events::ACTION)] {
            let Some(export) = module.get_export("on_action") else {
                continue;
            };

            match export {
                ExternType::Func(_func) => {
                    events |= event;
                }
                _ => {
                    tracing::warn!(
                        "Found event extern {} in module, but it is a {}",
                        name,
                        match export {
                            ExternType::Func(_) => unreachable!(),
                            ExternType::Global(_) => "Global",
                            ExternType::Table(_) => "Table",
                            ExternType::Memory(_) => "Memory",
                        }
                    );
                }
            }
        }

        Ok(Self { module, events })
    }
}
