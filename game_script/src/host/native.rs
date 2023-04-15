use std::ffi::OsStr;

use libloading::{Error, Library};

type MainFn = unsafe extern "C" fn();

pub struct NativeHost {}

#[derive(Debug)]
pub struct NativeScript {
    lib: Library,
}

impl NativeScript {
    pub fn new<P: AsRef<OsStr>>(path: P) -> Result<Self, Error> {
        let lib = unsafe { Library::new(path)? };
        Ok(Self { lib })
    }

    pub fn run(&self) {
        let Ok(symbol) = (unsafe { self.lib.get::<MainFn>(b"script_main") }) else {
              tracing::error!("native script does not contain a script_main symbol");
              return;
        };

        unsafe {
            symbol();
        }
    }
}
