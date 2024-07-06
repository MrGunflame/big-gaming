use std::fmt::{self, Debug, Display, Formatter, Write};

use thiserror::Error;
use wasmtime::{Engine, ExternType, Module, ValType};

const EXPORT_FUNCTIONS: &[FunctionExport] = &[
    FunctionExport {
        name: "on_init",
        params: &[],
        results: &[],
    },
    FunctionExport {
        name: "__wasm_fn_trampoline",
        params: &[ValType::I32, ValType::I64],
        results: &[],
    },
];

#[derive(Copy, Clone, Debug)]
struct FunctionExport {
    name: &'static str,
    params: &'static [ValType],
    results: &'static [ValType],
}

#[derive(Debug, Error)]
pub enum ScriptLoadError {
    #[error("module error: {0}")]
    Module(wasmtime::Error),
    #[error("instance init failed: {0}")]
    Init(wasmtime::Error),
}

pub(crate) struct Script {
    pub module: Module,
}

impl Script {
    pub fn new(bytes: &[u8], engine: &Engine) -> Result<Self, ScriptLoadError> {
        let module = Module::new(engine, bytes).map_err(ScriptLoadError::Module)?;

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

        Ok(Self { module })
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
        if !ValType::eq(&a, b) {
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
