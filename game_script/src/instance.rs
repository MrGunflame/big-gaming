use std::mem::MaybeUninit;

use game_common::entity::EntityId;
use game_common::world::entity::EntityBody;
use game_common::world::world::WorldState;
use game_wasm::log::Level;
use game_wasm::raw;
use game_wasm::raw::world::{Entity, Item};
use wasmtime::{
    AsContextMut, Caller, Engine, Func, FuncType, Instance, Linker, Module, Store, TypedFunc, Val,
};

use crate::events::{Events, OnAction, OnCollision};

macro_rules! register_fns {
    ($linker:expr, $($id:ident),*$(,)?) => {
        $(
            $linker.func_wrap("host", stringify!($id), $id);
        )*
    };
}

pub struct ScriptInstance<'a> {
    store: Store<State<'a>>,
    inner: Instance,
    events: Events,
}

impl<'a> ScriptInstance<'a> {
    pub fn new(
        engine: &Engine,
        module: &Module,
        events: Events,
        world: &'a mut WorldState,
    ) -> Self {
        let mut store = Store::new(engine, State { world });

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

        let mut linker = Linker::<State>::new(&engine);

        // linker.func_wrap(
        //     "host",
        //     "hello",
        //     |caller: Caller<'_, ()>, param: u32| -> u32 {
        //         println!("got {} hello world ", param);
        //         0
        //     },
        // );

        register_fns! {
            linker,
            log,
            world_entity_get,
            world_entity_spawn,
            world_entity_despawn,
        };

        let instance = linker.instantiate(&mut store, &module).unwrap();

        Self {
            store,
            inner: instance,
            events,
        }
    }

    // pub fn run(&mut self) {
    //     let main: TypedFunc<u32, u32> = self
    //         .inner
    //         .get_typed_func(&mut self.store, "script_main")
    //         .unwrap();

    //     dbg!("calling");
    //     let out = main.call(&mut self.store, 23).unwrap();
    //     dbg!(out);
    // }

    pub fn on_action(&mut self, entity: EntityId, invoker: EntityId) {
        let func: OnAction = self
            .inner
            .get_typed_func(&mut self.store, "on_action")
            .unwrap();

        func.call(&mut self.store, (entity.into_raw(), invoker.into_raw()))
            .unwrap();
    }

    pub fn on_collision(&mut self, entity: EntityId, other: EntityId) {
        let func: OnCollision = self
            .inner
            .get_typed_func(&mut self.store, "on_collision")
            .unwrap();

        func.call(&mut self.store, (entity.into_raw(), other.into_raw()))
            .unwrap();
    }
}

fn log(mut caller: Caller<'_, State<'_>>, level: u32, ptr: u32, len: u32) {
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

fn world_entity_spawn(mut caller: Caller<'_, State<'_>>, ptr: u32) -> u32 {
    0
}

fn world_entity_get(mut caller: Caller<'_, State<'_>>, id: u64, out: u32) -> u32 {
    let Some(view) = caller.data_mut().world.front() else {
        return 1;
    };

    let Some(entity) = view.get(EntityId::from_raw(id)) else {
        return 1;
    };

    let ent = Entity {
        id,
        translation: entity.transform.translation.to_array(),
        rotation: entity.transform.rotation.to_array(),
        scale: entity.transform.scale.to_array(),
        body: match &entity.body {
            EntityBody::Item(item) => raw::world::EntityBody::Item(Item { id: item.id.0 .0 }),
            _ => todo!(),
        },
    };

    let bytes = unsafe { std::mem::transmute::<Entity, [u8; std::mem::size_of::<Entity>()]>(ent) };

    let memory = caller.get_export("memory").unwrap().into_memory().unwrap();

    let buf = memory.data_mut(caller.as_context_mut());

    let buf = &mut buf[out as usize..out as usize + std::mem::size_of::<Entity>()];
    buf.copy_from_slice(&bytes);

    0
}

fn world_entity_despawn(mut caller: Caller<'_, State<'_>>, id: u64) -> u32 {
    let id = EntityId::from_raw(id);

    let Some(mut view) = caller.data_mut().world.front_mut() else {
        return 1;
    };

    view.despawn(id);
    0
}

pub struct State<'a> {
    world: &'a mut WorldState,
}
