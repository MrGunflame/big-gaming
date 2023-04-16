use game_wasm::log::Level;
use wasmtime::{Caller, Engine, Func, Instance, Linker, Module, Store, TypedFunc};

pub struct ScriptInstance {
    store: Store<()>,
    inner: Instance,
}

impl ScriptInstance {
    pub fn new(engine: &Engine, module: &Module) -> Self {
        let mut store = Store::new(engine, ());

        //  let imports = vec![Func::new(
        //      &store,
        //      FuncType::new(ValType::I32, ValType::I32),
        //      |caller, params, results| {
        //          dbg!(params);
        //          Ok(())
        //      },
        //  )];

        //  let imports = vec![Func::wrap(&mut store, |i: i32| -> i32 {
        //      dbg!(i);
        //      i * 2
        //  })];

        let mut linker = Linker::new(&engine);

        linker.func_wrap(
            "host",
            "hello",
            |caller: Caller<'_, ()>, param: u32| -> u32 {
                println!("got {} hello world ", param);
                0
            },
        );

        linker.func_wrap("host", "log", log);

        let instance = linker.instantiate(&mut store, &module).unwrap();

        Self {
            store,
            inner: instance,
        }
    }

    pub fn run(&mut self) {
        let main: TypedFunc<u32, u32> = self
            .inner
            .get_typed_func(&mut self.store, "script_main")
            .unwrap();

        dbg!("calling");
        let out = main.call(&mut self.store, 23).unwrap();
        dbg!(out);
    }
}

fn log(mut caller: Caller<'_, ()>, level: u32, ptr: u32, len: u32) {
    let mut memory = caller.get_export("memory").unwrap().into_memory().unwrap();

    let bytes = memory.data(&caller);

    let mut buf = &bytes[ptr as usize..ptr as usize + len as usize];

    let content = std::str::from_utf8(buf).unwrap();

    match Level::from_raw(level) {
        Level::ERROR => {
            tracing::error!("{}", content)
        }
        Level::WARN => {
            tracing::warn!("{}", content);
        }
        Level::INFO => {
            tracing::info!("{}", content)
        }
        Level::DEBUG => {
            tracing::info!("{}", content)
        }
        Level::TRACE => {
            tracing::info!("{}", content)
        }
        _ => unreachable!(),
    }
}
