use std::fmt::{self, Debug, Display, Formatter, Write};
use std::fs::File;
use std::io::Read;
use std::path::Path;

use wasmtime::{Engine, ExternType, Module, ValType};

use crate::events::Events;

const EXPORT_FUNCTIONS: &[FunctionExport] = &[
    FunctionExport {
        name: "on_init",
        event: Events::NONE,
        params: &[],
        results: &[],
    },
    FunctionExport {
        name: "on_action",
        event: Events::ACTION,
        params: &[ValType::I64],
        results: &[],
    },
    FunctionExport {
        name: "on_update",
        event: Events::UPDATE,
        params: &[ValType::I64],
        results: &[],
    },
];

#[derive(Copy, Clone, Debug)]
struct FunctionExport {
    event: Events,
    name: &'static str,
    params: &'static [ValType],
    results: &'static [ValType],
}

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

        for fn_sig in EXPORT_FUNCTIONS {
            let Some(export) = module.get_export(fn_sig.name) else {
                continue;
            };

            match export {
                ExternType::Func(func) => {
                    if !equal_params(func.params(), fn_sig.params)
                        || !equal_params(func.results(), fn_sig.results)
                    {
                        tracing::warn!(
                            "extern function {} has invalid signature (expected {}, found {})",
                            fn_sig.name,
                            display_function_sig(
                                fn_sig.name,
                                fn_sig.params.iter(),
                                fn_sig.results.iter()
                            ),
                            display_function_sig(fn_sig.name, func.params(), func.results()),
                        );
                        continue;
                    }

                    events |= fn_sig.event;
                }
                _ => {
                    tracing::warn!(
                        "Found event extern {} in module, but it is a {}",
                        fn_sig.name,
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

impl Debug for Script {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Script").finish_non_exhaustive()
    }
}

fn equal_params<T>(lhs: T, rhs: &'static [ValType]) -> bool
where
    T: ExactSizeIterator<Item = ValType>,
{
    if lhs.len() != rhs.len() {
        return false;
    }

    for (a, b) in lhs.zip(rhs.iter()) {
        if a != *b {
            return false;
        }
    }

    true
}

fn display_function_sig(
    name: &'static str,
    mut params: impl ExactSizeIterator<Item = impl Display>,
    mut results: impl ExactSizeIterator<Item = impl Display>,
) -> String {
    let mut buf = format!("fn {} (", name);

    if let Some(param) = params.next() {
        let _ = write!(buf, "{}", param);
    }
    for param in params {
        let _ = write!(buf, ", {}", param);
    }

    match results.len() {
        0 => {
            let _ = write!(buf, ")");
        }
        1 => {
            let _ = write!(buf, ") -> {}", results.next().unwrap());
        }
        _ => {
            let _ = write!(buf, ") -> (");

            let _ = write!(buf, "{}", results.next().unwrap());
            for result in results {
                let _ = write!(buf, ", {}", result);
            }

            let _ = write!(buf, ")");
        }
    }

    buf
}
