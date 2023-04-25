use std::ffi::OsStr;
use std::fs::File;
use std::io::Read;

use wasmtime::{Engine, ExternType, Module};

use crate::events::Events;

pub struct WasmScript {
    pub module: Module,
    pub events: Events,
}

impl WasmScript {
    pub fn new<P: AsRef<OsStr>>(p: P, engine: &Engine) -> Result<Self, Box<dyn std::error::Error>> {
        let mut file = File::open(p.as_ref())?;

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

    pub fn run(&self) {}
}

// pub struct ScriptContext {
//     store: Store<()>,
//     imports: Vec<Extern>,
//     instance: Instance,
// }

// impl ScriptContext {
//     pub fn new(engine: &Engine, module: &Module) {
//         let mut store = Store::new(engine, ());

//         //  let imports = vec![Func::new(
//         //      &store,
//         //      FuncType::new(ValType::I32, ValType::I32),
//         //      |caller, params, results| {
//         //          dbg!(params);
//         //          Ok(())
//         //      },
//         //  )];

//         let imports = vec![Func::wrap(&store, |i: i32| -> i32 {
//             dbg!(i);
//             i * 2
//         })];

//         let mut linker = Linker::new(&engine);

//         linker.func_wrap("host", "hello", |caller: Caller<'_, u32>, param: u32| {
//             dbg!("hello world ", param);
//         });

//         let instance = linker.instantiate(&mut store, &module).unwrap();

//         let hello = instance.get_typed_func(&mut store, "script_main").unwrap();

//         hello.call(&mut store, ());

//         //  let instance = Instance::new(&mut store, &module, &imports).unwrap();
//     }
// }
